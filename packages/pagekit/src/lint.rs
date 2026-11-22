//! Biome's lint and formatter, in process.
//!
//! The page's own sources (the `.ts` and `.tsx` files in the entry's folder, `vendor` and the
//! `[wasm] glue` left out) are linted and, with `fix`, safe-fixed and formatted in place. The
//! mapped packages of `[imports]` are linted read-only under the preset, their findings
//! notes: a fix goes where the package lives. The preset is Biome's `recommended` (no
//! `nursery`, see `DOMAINS`) with the classic JSX runtime under Preact's `h` and `Fragment`;
//! `[lint]` moves rules on, off or to notes (level `notice`) ([`Rules`]). Unsafe fixes are never applied,
//! except `SAFE_HERE`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, bail};
use biome_analyze::options::{JsxRuntime, PreferredQuote};
use biome_analyze::{
    ActionFilter, AnalysisFilter, AnalyzerConfiguration, AnalyzerOptions, ControlFlow,
    GroupCategory, Never, Queryable, RegistryVisitor, Rule, RuleCategoriesBuilder, RuleCategory,
    RuleDomain, RuleFilter, RuleGroup,
};
use biome_diagnostics::{Applicability, Diagnostic, PrintDescription};
use biome_formatter::{IndentStyle, IndentWidth, LineWidth, QuoteStyle};
use biome_js_analyze::JsAnalyzerServices;
use biome_js_formatter::context::{JsFormatOptions, Semicolons, TrailingCommas};
use biome_js_parser::{JsParserOptions, parse};
use biome_js_syntax::{AnyJsRoot, JsIdentifierBinding, JsImport, JsLanguage};
use biome_languages::JsFileSource;
use biome_rowan::{AstNode, TextSize};
use rayon::prelude::*;
use serde::Deserialize;

use crate::Page;

/// The `[lint]` table: Biome rules moved by group and by name. A rule's level is the
/// preset's, then its group's here, then its own here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// `style = "error"`: every rule of the group (`nursery` excepted) at that level.
    pub groups: BTreeMap<String, Level>,
    /// `"style/useBlockStatements" = "error"`, as a finding names it (a `lint/` in front is
    /// allowed). A `nursery` rule is turned on here only.
    pub rules: BTreeMap<String, Level>,
}

/// What a `[lint]` entry does to a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Runs and fails `lint` and `build`.
    Error,
    /// Runs, is printed and fails nothing.
    Notice,
    /// Does not run.
    Off,
}

/// One thing Biome has to say about a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The file, relative to the page.
    pub file: PathBuf,
    /// 1-based.
    pub line: usize,
    /// Biome's category (`lint/a11y/noSvgWithoutTitle`), `parse` for a syntax error,
    /// [`FORMAT`] for the formatter.
    pub rule: String,
    /// The diagnostic's one-line description.
    pub message: String,
    /// Printed as `note:`, never failing: a `notice` by level, or a mapped package's.
    pub notice: bool,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = if self.notice { "note" } else { "error" };
        write!(
            f,
            "{}:{}: {level}: {}: {}",
            self.file.display(),
            self.line,
            self.rule,
            self.message
        )
    }
}

/// What linting a page produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Every finding: the page's own files first, then the mapped packages', in file order.
    /// Without `fix`, a file the formatter would change is a [`FORMAT`] finding.
    pub findings: Vec<Finding>,
    /// The page's own files the fix rewrote, relative to the page.
    pub formatted: Vec<PathBuf>,
}

impl Report {
    /// How many findings fail the verb.
    #[must_use]
    pub fn errors(&self) -> usize {
        self.findings.iter().filter(|f| !f.notice).count()
    }
}

/// One line per finding, then `formatted <file>` per rewritten file.
impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for finding in &self.findings {
            writeln!(f, "{finding}")?;
        }
        for file in &self.formatted {
            writeln!(f, "formatted {}", file.display())?;
        }
        Ok(())
    }
}

/// A report with errors is what fails a build.
impl std::error::Error for Report {}

/// The rule of a file the formatter would change, or could not print.
pub const FORMAT: &str = "lint/format";

/// The upper bound on safe fixes applied to one file, so a fix that does not make its
/// diagnostic go away cannot loop forever.
const MAX_FIXES: usize = 256;

/// Lint `page`: its own sources under `rules`, safe-fixed and formatted in place with `fix`;
/// its mapped packages read-only under the preset. One file per core.
pub fn page(page: &Page, rules: &Rules, fix: bool) -> Result<Report> {
    let root = &page.root;
    let skip: Vec<PathBuf> = page
        .vendor
        .iter()
        .chain(page.wasm.as_ref().and_then(|w| w.glue.as_ref()))
        .map(|p| root.join(p))
        .collect();
    let kept = |p: &PathBuf| !skip.iter().any(|s| p.starts_with(s));
    let own_dir = root.join(page.entry.parent().unwrap_or_else(|| Path::new("")));
    let own: Vec<PathBuf> = crate::css::files(&own_dir)?
        .into_iter()
        .filter(kept)
        .collect();
    let mut mapped = vec![];
    for target in page.imports.values().map(|p| root.join(p)) {
        if target.is_dir() {
            mapped.extend(crate::css::files(&target)?);
        } else if target.extension().is_some_and(|e| e == "ts" || e == "tsx") {
            mapped.push(target);
        }
    }
    mapped.sort();
    mapped.dedup();
    mapped.retain(|p| kept(p) && !own.contains(p));
    let preset = Rules::default();
    let jobs: Vec<(PathBuf, bool)> = own
        .into_iter()
        .map(|p| (p, true))
        .chain(mapped.into_iter().map(|p| (p, false)))
        .collect();
    let linted = jobs
        .into_par_iter()
        .map(|(path, own)| {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let file = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            let mut linted = if own {
                lint_text(&file, &text, fix, true, rules)
            } else {
                lint_text(&file, &text, false, false, &preset)
            };
            for f in linted.findings.iter_mut().filter(|_| !own) {
                f.notice = true;
            }
            if let Some(out) = &linted.text {
                if fix {
                    std::fs::write(&path, out)
                        .with_context(|| format!("write {}", path.display()))?;
                } else {
                    linted.findings.push(Finding {
                        file: file.clone(),
                        line: first_change(&text, out),
                        rule: FORMAT.to_owned(),
                        message: "not as the formatter would leave it; `onepage lint --fix` \
                                  rewrites it"
                            .to_owned(),
                        notice: false,
                    });
                }
            }
            Ok((file, linted))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut report = Report::default();
    for (file, linted) in linted {
        report.findings.extend(linted.findings);
        if fix && linted.text.is_some() {
            report.formatted.push(file);
        }
    }
    Ok(report)
}

/// What linting one file produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Linted {
    /// The findings that remain (after the safe fixes, when they were applied).
    pub findings: Vec<Finding>,
    /// The file as Biome leaves it, when that differs from what came in.
    pub text: Option<String>,
}

/// Lint one file's text. `file` names it in the findings and picks the dialect: `.tsx` is
/// TSX, anything else TypeScript. With `fix`, the safe fixes and the formatter are applied
/// first, so the findings point at the text written back; without it, `formatted` says
/// whether to run the formatter at all. A file that does not parse yields its syntax errors.
#[must_use]
pub fn lint_text(file: &Path, text: &str, fix: bool, formatted: bool, rules: &Rules) -> Linted {
    let source = if file.extension().is_some_and(|e| e == "tsx") {
        JsFileSource::tsx()
    } else {
        JsFileSource::ts()
    };
    let parsed = parse(text, source, JsParserOptions::default());
    if parsed.has_errors() {
        let findings = parsed
            .diagnostics()
            .iter()
            .map(|d| finding(file, text, d))
            .collect();
        return Linted {
            findings,
            text: None,
        };
    }
    let options = options(file);
    let mut filter = AnalysisFilter {
        categories: RuleCategoriesBuilder::default()
            .with_syntax()
            .with_lint()
            .build(),
        enabled_rules: Some(rules.enabled()),
        ..AnalysisFilter::default()
    };
    // useImportType's fix can turn a JSX factory import into `import { type h }` (when `h` is
    // also named in a type), which the TypeScript strip erases: the page paints nothing.
    // Where it would do that, the rule is off for the file.
    if source.variant().is_jsx() && !erases_factory(&parsed.tree()) {
        let fixed = apply_safe_fixes(parsed.tree(), &filter, &options, source);
        if erases_factory(&fixed) {
            filter.disabled_rules = IMPORT_TYPE;
        }
    }
    let mut findings = Vec::new();
    let (root, located_in) = if fix {
        let fixed = apply_safe_fixes(parsed.tree(), &filter, &options, source);
        formatted_tree(file, fixed, source, &mut findings)
    } else {
        (parsed.tree(), text.to_owned())
    };
    biome_js_analyze::analyze(
        &root,
        filter,
        &options,
        &[],
        services(&root, source),
        |signal| {
            if let Some(d) = signal.diagnostic() {
                let mut found = finding(file, &located_in, &d);
                found.notice = rules.is_notice(&found.rule);
                findings.push(found);
            }
            ControlFlow::<Never>::Continue(())
        },
    );
    let out = if fix {
        located_in
    } else if formatted {
        format(&root, source).unwrap_or_else(|why| {
            findings.push(format_failed(file, &why));
            text.to_owned()
        })
    } else {
        text.to_owned()
    };
    findings.sort_by_key(|f| f.line);
    Linted {
        findings,
        text: (out != text).then_some(out),
    }
}

/// `fixed` formatted and parsed again, with its text; on a formatter that fails, `fixed`
/// itself and a finding.
fn formatted_tree(
    file: &Path,
    fixed: AnyJsRoot,
    source: JsFileSource,
    findings: &mut Vec<Finding>,
) -> (AnyJsRoot, String) {
    let why = match format(&fixed, source) {
        Ok(out) => {
            let reparsed = parse(&out, source, JsParserOptions::default());
            if !reparsed.has_errors() {
                return (reparsed.tree(), out);
            }
            "its output does not parse".to_owned()
        },
        Err(why) => why,
    };
    findings.push(format_failed(file, &why));
    let unformatted = fixed.to_string();
    (fixed, unformatted)
}

/// The rule that must not touch a JSX factory import (see [`lint_text`]).
const IMPORT_TYPE: &[RuleFilter<'static>] = &[RuleFilter::Rule("style", "useImportType")];

/// Whether an import brings `h` or `Fragment` in for its type alone.
fn erases_factory(root: &AnyJsRoot) -> bool {
    let starts_with_type = |n: &biome_rowan::SyntaxNode<JsLanguage>, at: &str| {
        n.text_trimmed().to_string().starts_with(at)
    };
    root.syntax()
        .descendants()
        .filter(|n| JsImport::can_cast(n.kind()))
        .any(|import| {
            let all = starts_with_type(&import, "import type ");
            import
                .descendants()
                .filter_map(JsIdentifierBinding::cast)
                .any(|b| {
                    let node = b.syntax();
                    let name = node.text_trimmed();
                    (name == "h" || name == "Fragment")
                        && (all || node.parent().is_some_and(|p| starts_with_type(&p, "type ")))
                })
        })
}

/// The rules whose fix is applied although Biome marks it unsafe.
///
/// `noUnusedImports`: unsafe in general because an unused import's side effects still run,
/// but the TypeScript strip elides such an import anyway, so removing it cannot change the
/// page.
const SAFE_HERE: &[(&str, &str)] = &[("correctness", "noUnusedImports")];

/// Apply safe fixes one at a time until none is left: two fixes on one tree can overlap.
fn apply_safe_fixes(
    mut root: AnyJsRoot,
    filter: &AnalysisFilter<'_>,
    options: &AnalyzerOptions,
    source: JsFileSource,
) -> AnyJsRoot {
    for _ in 0..MAX_FIXES {
        let services = services(&root, source);
        let (action, _) = biome_js_analyze::analyze(&root, *filter, options, &[], services, |s| {
            for action in s.actions(ActionFilter::rule_fix()) {
                let safe = action.applicability == Applicability::Always
                    || action
                        .rule_name
                        .is_some_and(|rule| SAFE_HERE.contains(&rule));
                if action.is_suppression() || !safe {
                    continue;
                }
                return ControlFlow::Break(action);
            }
            ControlFlow::Continue(())
        });
        let Some(action) = action else { break };
        let next = AnyJsRoot::unwrap_cast(action.mutation.commit());
        // A fix that changed nothing would be found again on the next pass.
        if next.to_string() == root.to_string() {
            break;
        }
        root = next;
    }
    root
}

/// The analyzer's options for `file`: its path, the classic JSX runtime with Preact's
/// factory names, and double quotes as the formatter prints them.
fn options(file: &Path) -> AnalyzerOptions {
    let configuration = AnalyzerConfiguration::default()
        .with_jsx_runtime(JsxRuntime::ReactClassic)
        .with_jsx_factory(Some("h".into()))
        .with_jsx_fragment_factory(Some("Fragment".into()))
        .with_preferred_quote(PreferredQuote::Double)
        .with_preferred_jsx_quote(PreferredQuote::Double);
    AnalyzerOptions::default()
        .with_file_path(file.to_string_lossy().as_ref())
        .with_configuration(configuration)
}

/// What the analyzer may consult beyond the tree: the dialect, and an empty
/// embedded-document index - some rules panic without one.
fn services(root: &AnyJsRoot, source: JsFileSource) -> JsAnalyzerServices {
    static NONE: OnceLock<Arc<biome_embeds::EmbeddedData>> = OnceLock::new();
    JsAnalyzerServices::from(root)
        .with_source_type(source)
        .with_embedded_data(Some(NONE.get_or_init(Default::default).clone()))
}

/// The finding for a formatter that failed on `file`.
fn format_failed(file: &Path, why: &str) -> Finding {
    Finding {
        file: file.to_path_buf(),
        line: 1,
        rule: FORMAT.to_owned(),
        message: format!("the formatter failed on this file: {why}"),
        notice: false,
    }
}

/// The tree printed: two spaces, 100 columns, double quotes, trailing commas, semicolons.
fn format(root: &AnyJsRoot, source: JsFileSource) -> Result<String, String> {
    let options = JsFormatOptions::new(source)
        .with_indent_style(IndentStyle::Space)
        .with_indent_width(IndentWidth::try_from(2).unwrap_or_default())
        .with_line_width(LineWidth::try_from(100).unwrap_or_default())
        .with_quote_style(QuoteStyle::Double)
        .with_jsx_quote_style(QuoteStyle::Double)
        .with_trailing_commas(TrailingCommas::All)
        .with_semicolons(Semicolons::Always);
    let printed = biome_js_formatter::format_node(options, root.syntax(), Vec::new())
        .map_err(|e| e.to_string())?
        .print()
        .map_err(|e| e.to_string())?;
    Ok(printed.into_code())
}

/// A finding from a diagnostic, located in `text`.
fn finding<D: Diagnostic>(file: &Path, text: &str, d: &D) -> Finding {
    Finding {
        file: file.to_path_buf(),
        line: d
            .location()
            .span
            .map_or(1, |span| line_at(text, span.start())),
        rule: d.category().map_or("unknown", |c| c.name()).to_owned(),
        // One line: some descriptions list their suggestions on lines of their own.
        message: PrintDescription(d)
            .to_string()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        notice: false,
    }
}

/// The 1-based line of a byte offset.
fn line_at(text: &str, at: TextSize) -> usize {
    let at = usize::from(at).min(text.len());
    text[..at].matches('\n').count() + 1
}

/// The 1-based line where `a` and `b` first differ.
fn first_change(a: &str, b: &str) -> usize {
    let (mut a, mut b) = (a.lines(), b.lines());
    let mut line = 1;
    while let (Some(x), Some(y)) = (a.next(), b.next()) {
        if x != y {
            break;
        }
        line += 1;
    }
    line
}

/// The rule domains that are on: `React` (Preact follows its rules) and `Test`. A domain rule
/// runs when ANY domain it names is here, as in Biome. `Project` is off: it needs a module
/// graph a lone file does not have.
const DOMAINS: &[RuleDomain] = &[RuleDomain::React, RuleDomain::Test];

/// Recommended rules reported as notes in the preset, never failing.
///
/// - `useExhaustiveDependencies`: an extra dependency is often deliberate, to re-run an
///   effect, and the rule cannot tell that from a mistake.
/// - `useJsxKeyInIterable`: flags every JSX element in any array literal, whatever the array
///   is for.
const NOTICE: &[(&str, &str)] = &[
    ("correctness", "useExhaustiveDependencies"),
    ("correctness", "useJsxKeyInIterable"),
];

/// A finding's `lint/<group>/<name>`, split; `None` for anything else.
fn of_finding(rule: &str) -> Option<(&str, &str)> {
    rule.strip_prefix("lint/").and_then(|r| r.split_once('/'))
}

/// The rules a lint runs and which of them are notes: the preset, or a page's `[lint]` on
/// top of it. `Default` is the preset, exactly.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    /// What runs; `None` is [`recommended`].
    enabled: Option<Vec<RuleFilter<'static>>>,
    /// The level the page gave each rule it moved, by `(group, name)`.
    levels: BTreeMap<(&'static str, &'static str), Level>,
}

impl Rules {
    /// The rules `config` asks for: the preset, then each group's level, then each rule's.
    ///
    /// A group or rule Biome does not have (named), a `nursery` group, or a rule that needs
    /// the project's module graph is an error.
    pub fn of(config: &Config) -> Result<Self> {
        if config.groups.is_empty() && config.rules.is_empty() {
            return Ok(Self::default());
        }
        let known = registry();
        let mut levels = BTreeMap::new();
        for (group, &level) in &config.groups {
            if group == "nursery" {
                bail!(
                    "groups.nursery: a nursery rule is unfinished and is turned on by name \
                     only, in `rules`"
                );
            }
            if !known.iter().any(|k| k.lint && k.group == group) {
                bail!(
                    "groups.{group}: Biome has no such group; one of {}",
                    groups().join(", ")
                );
            }
            for k in known
                .iter()
                .filter(|k| k.lint && k.group == group && k.grouped())
            {
                levels.insert((k.group, k.name), level);
            }
        }
        for (key, &level) in &config.rules {
            let Some((group, name)) = key.strip_prefix("lint/").unwrap_or(key).split_once('/')
            else {
                bail!(
                    "rules.\"{key}\": a rule is `<group>/<name>`, as a finding names it: \
                     `style/useBlockStatements`"
                );
            };
            let Some(k) = known
                .iter()
                .find(|k| k.lint && k.group == group && k.name == name)
            else {
                if known.iter().any(|k| k.lint && k.group == group) {
                    bail!("rules.\"{key}\": Biome has no rule `{name}` in `{group}`");
                }
                bail!(
                    "rules.\"{key}\": Biome has no group `{group}`; one of {}",
                    groups().join(", ")
                );
            };
            if k.project {
                bail!(
                    "rules.\"{key}\": the rule reads the project's module graph, which a lint \
                     of one file at a time does not have"
                );
            }
            levels.insert((k.group, k.name), level);
        }
        let on = |k: &Known| {
            levels
                .get(&(k.group, k.name))
                .map_or(k.preset, |level| *level != Level::Off)
        };
        let enabled = known
            .iter()
            .filter(|k| on(k))
            .map(|k| RuleFilter::Rule(k.group, k.name))
            .collect();
        Ok(Self {
            enabled: Some(enabled),
            levels,
        })
    }

    /// The analyzer's rule list.
    fn enabled(&self) -> &[RuleFilter<'static>] {
        match &self.enabled {
            Some(rules) => rules,
            None => recommended(),
        }
    }

    /// Whether a finding's rule is a notice: the page's level, else the preset's `NOTICE`.
    #[must_use]
    pub fn is_notice(&self, rule: &str) -> bool {
        of_finding(rule).is_some_and(|at| {
            self.levels
                .get(&at)
                .map_or_else(|| NOTICE.contains(&at), |level| *level == Level::Notice)
        })
    }
}

/// Every rule Biome's registry marks recommended, `nursery` and the domains outside
/// [`DOMAINS`] excluded, plus the syntax rules. An `AnalysisFilter` with no rule list would
/// run every rule, nursery and all.
fn recommended() -> &'static [RuleFilter<'static>] {
    static RULES: OnceLock<Vec<RuleFilter<'static>>> = OnceLock::new();
    RULES.get_or_init(|| {
        registry()
            .iter()
            .filter(|k| k.preset)
            .map(|k| RuleFilter::Rule(k.group, k.name))
            .collect()
    })
}

/// Every lint group's name, `nursery` left out.
fn groups() -> Vec<&'static str> {
    let mut groups: Vec<&str> = registry()
        .iter()
        .filter(|k| k.lint && k.group != "nursery")
        .map(|k| k.group)
        .collect();
    groups.dedup();
    groups
}

/// One rule in Biome's registry, as [`Rules`] reads it.
#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is one fact the registry states about the rule"
)]
struct Known {
    group: &'static str,
    name: &'static str,
    /// A lint rule; else a syntax rule, which always runs and a page cannot name.
    lint: bool,
    /// In the preset ([`recommended`]).
    preset: bool,
    /// Runs under [`DOMAINS`] and is not deprecated: what a group switch turns on.
    in_domains: bool,
    /// Needs the project's module graph (the `Project` domain).
    project: bool,
}

impl Known {
    /// Whether a group's switch moves this rule: what the preset runs, and every other rule
    /// that runs under [`DOMAINS`].
    const fn grouped(&self) -> bool {
        (self.preset || self.in_domains) && !self.project
    }
}

/// Biome's registry, in its own order: the syntax rules and every lint rule.
fn registry() -> &'static [Known] {
    static RULES: OnceLock<Vec<Known>> = OnceLock::new();
    RULES.get_or_init(|| {
        let mut visitor = Registry {
            category: RuleCategory::Lint,
            rules: Vec::new(),
        };
        biome_js_analyze::visit_registry(&mut visitor);
        visitor.rules
    })
}

/// The registry walk behind [`registry`].
struct Registry {
    /// The category of the group being recorded; the registry announces it before the
    /// groups under it.
    category: RuleCategory,
    rules: Vec<Known>,
}

impl RegistryVisitor<JsLanguage> for Registry {
    fn record_category<C: GroupCategory<Language = JsLanguage>>(&mut self) {
        self.category = C::CATEGORY;
        C::record_groups(self);
    }

    fn record_rule<R>(&mut self)
    where
        R: Rule<Query: Queryable<Language = JsLanguage, Output: Clone>> + 'static,
    {
        let group = <R::Group as RuleGroup>::NAME;
        let name = R::METADATA.name;
        let domains = R::METADATA.domains;
        let under_domains = group != "nursery"
            && (domains.is_empty() || domains.iter().any(|d| DOMAINS.contains(d)));
        let known = match self.category {
            RuleCategory::Syntax => Known {
                group,
                name,
                lint: false,
                preset: true,
                in_domains: false,
                project: false,
            },
            RuleCategory::Lint => Known {
                group,
                name,
                lint: true,
                preset: R::METADATA.recommended && under_domains,
                in_domains: under_domains && R::METADATA.deprecated.is_none(),
                project: domains.contains(&RuleDomain::Project),
            },
            RuleCategory::Action | RuleCategory::Transformation => return,
        };
        self.rules.push(known);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lint(text: &str, fix: bool, rules: &Rules) -> Linted {
        lint_text(Path::new("src/app.tsx"), text, fix, true, rules)
    }

    #[test]
    fn a_finding_names_its_file_line_and_rule() {
        let linted = lint(
            "export const a = 1;\n\ndebugger;\n",
            false,
            &Rules::default(),
        );
        let finding = &linted.findings[0];
        assert_eq!(finding.rule, "lint/suspicious/noDebugger");
        assert_eq!(
            finding.to_string(),
            format!(
                "src/app.tsx:3: error: lint/suspicious/noDebugger: {}",
                finding.message
            )
        );
    }

    #[test]
    fn a_fix_rewrites_and_is_idempotent() {
        let messy = "import {h} from 'preact'\nexport const A = () => <p class='x'>hi</p>\n";
        let fixed = lint(messy, true, &Rules::default()).text.unwrap();
        assert_eq!(
            fixed,
            "import { h } from \"preact\";\nexport const A = () => <p class=\"x\">hi</p>;\n"
        );
        let again = lint(&fixed, true, &Rules::default());
        assert_eq!(again.text, None);
        assert!(again.findings.is_empty(), "{:?}", again.findings);
    }

    #[test]
    fn a_rule_turned_off_does_not_run_and_a_group_turns_rules_on() {
        let text = "debugger;\n";
        assert_eq!(lint(text, false, &Rules::default()).findings.len(), 1);
        let off = Config {
            rules: [("suspicious/noDebugger".into(), Level::Off)].into(),
            ..Config::default()
        };
        assert_eq!(lint(text, false, &Rules::of(&off).unwrap()).findings, []);
        let notice = Config {
            rules: [("lint/suspicious/noDebugger".into(), Level::Notice)].into(),
            ..Config::default()
        };
        assert!(lint(text, false, &Rules::of(&notice).unwrap()).findings[0].notice);
        // `noMagicNumbers` is in `style` and not recommended: on with the group only.
        let magic = "export const f = (x: number) => x * 37;\n";
        let rule = "lint/style/noMagicNumbers";
        let has = |r: &Rules| {
            lint(magic, false, r)
                .findings
                .iter()
                .any(|f| f.rule == rule)
        };
        assert!(!has(&Rules::default()));
        let style = Config {
            groups: [("style".into(), Level::Error)].into(),
            ..Config::default()
        };
        assert!(has(&Rules::of(&style).unwrap()));
    }

    #[test]
    fn an_unknown_rule_or_group_is_reported() {
        let rule = Config {
            rules: [("suspicious/noVra".into(), Level::Off)].into(),
            ..Config::default()
        };
        let error = Rules::of(&rule).unwrap_err().to_string();
        assert_eq!(
            error,
            "rules.\"suspicious/noVra\": Biome has no rule `noVra` in `suspicious`"
        );
        let group = Config {
            groups: [("stlye".into(), Level::Error)].into(),
            ..Config::default()
        };
        let error = Rules::of(&group).unwrap_err().to_string();
        assert!(
            error.starts_with("groups.stlye: Biome has no such group"),
            "{error}"
        );
    }

    #[test]
    fn a_syntax_error_is_a_parse_finding() {
        let linted = lint("export const a = ;\n", true, &Rules::default());
        assert_eq!(linted.text, None);
        assert_eq!(linted.findings[0].rule, "parse");
        assert_eq!(linted.findings[0].line, 1);
    }
}
