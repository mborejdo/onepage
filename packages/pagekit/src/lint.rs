//! Oxc's linter (Oxlint's rules) and formatter, in process.
//!
//! The page's own sources (the `.ts` and `.tsx` files in the entry's folder, `vendor` and the
//! `[wasm] glue` left out) are linted and, with `fix`, safe-fixed and formatted in place. The
//! mapped packages of `[imports]` are linted read-only under the preset, their findings
//! notes: a fix goes where the package lives.
//!
//! The preset is onepage's own list of Oxlint rules (`lint-preset.txt`). `[lint]` moves rules
//! on, off or to notes (level `notice`) ([`Rules`]): by Oxlint category and by Oxlint rule
//! name, with the rule's options where it takes them. Only safe fixes are applied, and the
//! removal of an unused import.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_diagnostics::Severity;
use oxc_formatter::{JsFormatOptions, QuoteStyle, Semicolons, TrailingCommas};
use oxc_formatter_core::{IndentStyle, IndentWidth, LineWidth};
use oxc_linter::{
    ConfigStore, ConfigStoreBuilder, ExternalPluginStore, FixKind, Fixer, LintOptions, LintService,
    LintServiceOptions, Linter, Message, Oxlintrc, RuntimeFileSystem,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Page, line_at};

/// The `[lint]` table: rules moved by Oxlint category and by name. A rule's level is the
/// preset's, then its category's here, then its own here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// `style = "error"`: every rule of an Oxlint category (`nursery` excepted) in the preset's
    /// plugins, at that level.
    pub categories: BTreeMap<String, Level>,
    /// `"eslint/no-debugger" = "off"`, by the name a finding gives the rule; or the level and
    /// the rule's options, as Oxlint takes them: `"eslint/max-params" = ["error", { max = 4 }]`.
    pub rules: BTreeMap<String, Value>,
    /// Not a key: read only to say that `categories` took its place.
    #[doc(hidden)]
    pub groups: Option<Value>,
}

/// A `[lint]` entry that cannot run: the name it is about and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// The rule, category or key, as written.
    pub name: String,
    /// The one-line reason, starting at the key (`rules: ...`).
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// A [`ConfigError`] about `name`.
fn mistake(name: &str, message: String) -> ConfigError {
    ConfigError {
        name: name.to_owned(),
        message,
    }
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

/// One thing the linter or the formatter has to say about a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The file, relative to the page.
    pub file: PathBuf,
    /// 1-based.
    pub line: usize,
    /// The Oxlint rule (`eslint/no-debugger`), [`PARSE`] for a syntax error, [`FORMAT`] for
    /// the formatter.
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

/// One line per finding.
impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for finding in &self.findings {
            writeln!(f, "{finding}")?;
        }
        Ok(())
    }
}

/// A report with errors is what fails a build.
impl std::error::Error for Report {}

/// The rule of a file the formatter would change, or could not print.
pub const FORMAT: &str = "format";

/// The rule of a syntax error.
pub const PARSE: &str = "parse";

/// The upper bound on fix passes over one file, so a fix that does not make its diagnostic
/// go away cannot loop forever.
const MAX_FIXES: usize = 16;

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
    let own_linter = Engine::new(rules)?;
    let preset_linter = Engine::new(&Rules::default())?;
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
                own_linter.lint(&file, &text, fix, true)
            } else {
                preset_linter.lint(&file, &text, false, false)
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
    /// The file as the fixes and the formatter leave it, when that differs from what came in.
    pub text: Option<String>,
}

/// Lint one file's text. `file` names it in the findings and picks the dialect: `.tsx` is
/// TSX, anything else TypeScript. With `fix`, the safe fixes and the formatter are applied
/// first, so the findings point at the text written back; without it, `formatted` says
/// whether to run the formatter at all. A file that does not parse yields its syntax errors.
pub fn lint_text(
    file: &Path,
    text: &str,
    fix: bool,
    formatted: bool,
    rules: &Rules,
) -> Result<Linted> {
    Ok(Engine::new(rules)?.lint(file, text, fix, formatted))
}

/// One configured Oxlint, shared by the files of a lint.
struct Engine {
    service: LintService,
    rules: Rules,
}

impl Engine {
    fn new(rules: &Rules) -> Result<Self> {
        let oxlintrc: Oxlintrc =
            serde_json::from_value(rules.oxlintrc()).context("parse the linter's configuration")?;
        let mut store = ExternalPluginStore::default();
        let config = ConfigStoreBuilder::from_oxlintrc(true, oxlintrc, None, &mut store, None)
            .and_then(|b| b.build(&mut store))
            .context("build the linter's configuration")?;
        let linter = Linter::new(
            LintOptions::default(),
            ConfigStore::new(config, rustc_hash::FxHashMap::default(), store),
            None,
        )
        .with_fix(FixKind::SafeFix);
        let service = LintService::new(linter, LintServiceOptions::new(PathBuf::from("/")));
        Ok(Self {
            service,
            rules: rules.clone(),
        })
    }

    /// The messages for `text`, as if it were the file at `file`.
    fn messages(&self, file: &Path, text: &str) -> Vec<Message> {
        // The linter picks its dialect from the path and reads the text from memory.
        let path = Path::new("/").join(file.file_name().unwrap_or_else(|| OsStr::new("file.ts")));
        let memory = Memory {
            path: path.clone(),
            text,
        };
        self.service
            .run_source(&memory, vec![Arc::<OsStr>::from(path.as_os_str())])
    }

    fn lint(&self, file: &Path, text: &str, fix: bool, formatted: bool) -> Linted {
        let source = source_type(file);
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, text, source).parse();
        if !parsed.diagnostics.is_empty() {
            let findings = parsed
                .diagnostics
                .iter()
                .map(|d| Finding {
                    file: file.to_path_buf(),
                    line: d.labels.first().map_or(1, |l| line_at(text, l.offset())),
                    rule: PARSE.to_owned(),
                    message: one_line(&d.message),
                    notice: false,
                })
                .collect();
            return Linted {
                findings,
                text: None,
            };
        }
        let mut findings = Vec::new();
        let mut out = text.to_owned();
        if fix {
            out = self.fixed(file, out);
            match format(&out, source) {
                Ok(printed) => out = printed,
                Err(why) => findings.push(format_failed(file, &why)),
            }
        }
        findings.extend(
            self.messages(file, &out)
                .iter()
                .map(|m| self.finding(file, &out, m)),
        );
        if !fix && formatted {
            match format(text, source) {
                Ok(printed) => out = printed,
                Err(why) => findings.push(format_failed(file, &why)),
            }
        }
        findings.sort_by_key(|f| f.line);
        Linted {
            findings,
            text: (out != text).then_some(out),
        }
    }

    /// `text` with the safe fixes applied, pass after pass until none is left.
    fn fixed(&self, file: &Path, mut text: String) -> String {
        for _ in 0..MAX_FIXES {
            // A note never rewrites the file: only the errors' fixes are applied.
            let messages: Vec<Message> = self
                .messages(file, &text)
                .into_iter()
                .filter(|m| m.error.severity == Severity::Error)
                .collect();
            if messages.iter().all(|m| m.fixes.is_empty()) {
                break;
            }
            let result = Fixer::new(&text, messages, Some(source_type(file))).fix();
            if !result.fixed || result.fixed_code == text {
                break;
            }
            text = result.fixed_code.into_owned();
        }
        text
    }

    fn finding(&self, file: &Path, text: &str, m: &Message) -> Finding {
        let code = &m.error.code;
        let rule = match (&code.scope, &code.number) {
            (Some(scope), Some(number)) => format!("{scope}/{number}"),
            _ => "unknown".to_owned(),
        };
        let notice = m.error.severity == Severity::Warning || self.rules.is_notice(&rule);
        Finding {
            file: file.to_path_buf(),
            line: line_at(text, m.span.start),
            rule,
            message: one_line(&m.error.message),
            notice,
        }
    }
}

/// The linter's file system: the one file being linted, from memory.
struct Memory<'t> {
    path: PathBuf,
    text: &'t str,
}

impl RuntimeFileSystem for Memory<'_> {
    fn read_to_arena_str<'a>(
        &self,
        path: &Path,
        allocator: &'a Allocator,
    ) -> Result<&'a str, std::io::Error> {
        if path == self.path {
            Ok(allocator.alloc_str(self.text))
        } else {
            Err(std::io::ErrorKind::NotFound.into())
        }
    }

    fn write_file(&self, _: &Path, _: &str) -> Result<(), std::io::Error> {
        Ok(())
    }
}

/// `.tsx` is TSX, anything else TypeScript.
fn source_type(file: &Path) -> SourceType {
    if file.extension().is_some_and(|e| e == "tsx") {
        SourceType::tsx()
    } else {
        SourceType::ts()
    }
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

/// The text printed: two spaces, 100 columns, double quotes, trailing commas, semicolons.
fn format(text: &str, source: SourceType) -> Result<String, String> {
    let options = JsFormatOptions {
        indent_style: IndentStyle::Space,
        indent_width: IndentWidth::try_from(2).unwrap_or_default(),
        line_width: LineWidth::try_from(100).unwrap_or_default(),
        quote_style: QuoteStyle::Double,
        jsx_quote_style: QuoteStyle::Double,
        trailing_commas: TrailingCommas::All,
        semicolons: Semicolons::Always,
        ..JsFormatOptions::default()
    };
    let allocator = Allocator::default();
    let formatted = oxc_formatter::format(&allocator, text, source, options)
        .map_err(|e| one_line(&e.message))?;
    let printed = formatted.print().map_err(|e| e.to_string())?.into_code();
    let reparsed = Parser::new(&allocator, &printed, source).parse();
    if reparsed.diagnostics.is_empty() {
        Ok(printed)
    } else {
        Err("its output does not parse".to_owned())
    }
}

/// A description on one line: some list their suggestions on lines of their own.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// The Oxlint categories a page may move whole. `nursery` is left out: its rules are
/// unfinished, and turned on by name only.
const CATEGORIES: &[&str] = &[
    "correctness",
    "suspicious",
    "pedantic",
    "perf",
    "style",
    "restriction",
];

/// The rules a lint runs and at which level: the preset, or a page's `[lint]` on top of it.
/// `Default` is the preset, exactly.
#[derive(Debug, Clone)]
pub struct Rules {
    /// Every rule that runs, by its Oxlint name, with its level (never `Off`).
    levels: BTreeMap<String, Level>,
    /// The options a page gave a rule (the elements after the level).
    options: BTreeMap<String, Vec<Value>>,
}

impl Default for Rules {
    fn default() -> Self {
        static PRESET: LazyLock<Rules> = LazyLock::new(|| Rules {
            levels: preset()
                .iter()
                .map(|(rule, level)| ((*rule).to_owned(), *level))
                .collect(),
            options: BTreeMap::new(),
        });
        PRESET.clone()
    }
}

impl Rules {
    /// The rules `config` asks for: the preset, then each category's level, then each rule's.
    ///
    /// # Errors
    ///
    /// A `groups` key, an unknown or `nursery` category, a rule Oxlint does not have (named),
    /// a rule that needs type information, or a level other than `error`, `notice` or `off`.
    pub fn of(config: &Config) -> Result<Self, ConfigError> {
        if config.groups.is_some() {
            return Err(mistake(
                "groups",
                "groups: no such key; Oxlint's categories go in `categories` \
                 (`categories = { style = \"error\" }`)"
                    .to_owned(),
            ));
        }
        let mut rules = Self::default();
        let plugins: BTreeSet<&str> = preset()
            .iter()
            .filter_map(|(rule, _)| rule.split_once('/').map(|(plugin, _)| plugin))
            .collect();
        for (category, &level) in &config.categories {
            if category == "nursery" {
                return Err(mistake(
                    category,
                    "categories: a nursery rule is unfinished; turn it on by name, in `rules`"
                        .to_owned(),
                ));
            }
            if !CATEGORIES.contains(&category.as_str()) {
                return Err(mistake(
                    category,
                    format!(
                        "categories: expected one of {}, got \"{category}\"",
                        choices(CATEGORIES.iter().copied())
                    ),
                ));
            }
            for rule in oxlint().iter().filter(|r| {
                r.category == category.as_str() && !r.type_aware && plugins.contains(r.plugin)
            }) {
                rules.set(&format!("{}/{}", rule.plugin, rule.name), level, vec![]);
            }
        }
        for (key, setting) in &config.rules {
            let (level, options) = setting_of(key, setting)?;
            let known = oxlint();
            let rule = key
                .split_once('/')
                .and_then(|(plugin, name)| {
                    known.iter().find(|r| r.plugin == plugin && r.name == name)
                })
                .ok_or_else(|| {
                    let plugins: BTreeSet<&str> = known.iter().map(|r| r.plugin).collect();
                    mistake(
                        key,
                        format!(
                            "rules: expected an Oxlint rule by the name a finding gives it \
                             (`eslint/no-debugger`), got \"{key}\"; the plugin is one of {}",
                            choices(plugins.into_iter())
                        ),
                    )
                })?;
            if rule.type_aware {
                return Err(mistake(
                    key,
                    format!("rules: \"{key}\" needs type information, which a lint does not have"),
                ));
            }
            rules.set(key, level, options);
        }
        Ok(rules)
    }

    /// Whether a finding's rule is a notice.
    #[must_use]
    pub fn is_notice(&self, rule: &str) -> bool {
        self.levels.get(rule) == Some(&Level::Notice)
    }

    fn set(&mut self, rule: &str, level: Level, options: Vec<Value>) {
        self.options.remove(rule);
        if level == Level::Off {
            self.levels.remove(rule);
            return;
        }
        self.levels.insert(rule.to_owned(), level);
        if !options.is_empty() {
            self.options.insert(rule.to_owned(), options);
        }
    }

    /// The configuration Oxlint reads: these rules only, nothing from its own defaults.
    fn oxlintrc(&self) -> Value {
        let mut plugins = BTreeSet::new();
        let mut rules = serde_json::Map::new();
        for (rule, level) in &self.levels {
            let severity = if *level == Level::Notice {
                "warn"
            } else {
                "error"
            };
            let (plugin, name) = rule.split_once('/').unwrap_or(("eslint", rule));
            plugins.insert(plugin_config_name(plugin));
            let mut value = vec![json!(severity)];
            if let Some(options) = self.options.get(rule) {
                value.extend(options.iter().cloned());
            } else if rule == "eslint/no-unused-vars" {
                // `_`-names are allowed, and so is a `Fragment` only `<>` uses (the classic
                // runtime reads it; Oxlint already knows `h`). An unused import is removed by
                // the fix: the TypeScript strip would elide it anyway.
                value.push(json!({
                    "varsIgnorePattern": "^(_|Fragment$)",
                    "argsIgnorePattern": "^_",
                    "caughtErrors": "none",
                    "fix": { "imports": "safe-fix", "variables": "suggestion" }
                }));
            }
            rules.insert(
                format!("{}/{name}", plugin_config_name(plugin)),
                Value::Array(value),
            );
        }
        json!({
            "plugins": plugins.into_iter().collect::<Vec<_>>(),
            "categories": {},
            "env": { "browser": true, "es2024": true },
            "rules": rules,
        })
    }
}

/// The level and the options of a `rules` entry: `"off"`, or `["error", { ... }]`.
fn setting_of(key: &str, setting: &Value) -> Result<(Level, Vec<Value>), ConfigError> {
    let (level, options) = match setting {
        Value::Array(items) => match items.split_first() {
            Some((level, options)) => (level, options.to_vec()),
            None => (setting, vec![]),
        },
        level => (level, vec![]),
    };
    let level = match level.as_str() {
        Some("error") => Level::Error,
        Some("notice") => Level::Notice,
        Some("off") => Level::Off,
        _ => {
            return Err(mistake(
                key,
                format!("rules: \"{key}\": expected error, notice or off, got {level}"),
            ));
        },
    };
    Ok((level, options))
}

/// `a, b or c`.
fn choices<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let items: Vec<&str> = items.collect();
    match items.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The preset, from `lint-preset.txt`: each rule with its level.
fn preset() -> &'static [(&'static str, Level)] {
    static PRESET: LazyLock<Vec<(&str, Level)>> = LazyLock::new(|| {
        include_str!("lint-preset.txt")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .filter_map(|l| {
                let mut columns = l.split_whitespace();
                let rule = columns.next()?;
                let level = if columns.next() == Some("notice") {
                    Level::Notice
                } else {
                    Level::Error
                };
                Some((rule, level))
            })
            .collect()
    });
    &PRESET
}

/// The name a plugin goes by in Oxlint's configuration.
fn plugin_config_name(display: &str) -> &str {
    if display == "next" { "nextjs" } else { display }
}

/// The name a plugin goes by in a finding.
const fn plugin_display_name(plugin: &str) -> &str {
    match plugin.as_bytes() {
        b"jsx_a11y" => "jsx-a11y",
        b"react_perf" => "react-perf",
        b"nextjs" => "next",
        _ => plugin,
    }
}

/// One rule in Oxlint's registry.
#[derive(Debug)]
struct Known {
    /// As a finding names it: `jsx-a11y`, not `jsx_a11y`.
    plugin: &'static str,
    name: &'static str,
    category: &'static str,
    type_aware: bool,
}

/// Oxlint's registry.
fn oxlint() -> &'static [Known] {
    static RULES: LazyLock<Vec<Known>> = LazyLock::new(|| {
        oxc_linter::rules::RULES
            .iter()
            .map(|r| Known {
                plugin: plugin_display_name(r.plugin_name()),
                name: r.name(),
                category: r.category().as_str(),
                type_aware: r.is_tsgolint_rule(),
            })
            .collect()
    });
    &RULES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lint(text: &str, fix: bool, rules: &Rules) -> Linted {
        lint_text(Path::new("src/app.tsx"), text, fix, true, rules).unwrap()
    }

    fn config(categories: &[(&str, Level)], rules: &[(&str, Value)]) -> Config {
        Config {
            categories: categories
                .iter()
                .map(|(k, l)| ((*k).to_owned(), *l))
                .collect(),
            rules: rules
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            groups: None,
        }
    }

    #[test]
    fn every_preset_rule_is_in_oxlint() {
        for (rule, _) in preset() {
            let (plugin, name) = rule.split_once('/').unwrap();
            let known = oxlint()
                .iter()
                .find(|r| r.plugin == plugin && r.name == name);
            assert!(known.is_some_and(|r| !r.type_aware), "{rule}");
        }
        assert!(Rules::default().levels.len() > 150);
        assert!(Rules::default().is_notice("react/exhaustive-deps"));
    }

    #[test]
    fn a_finding_names_its_file_line_and_rule() {
        let linted = lint(
            "export const a = 1;\n\ndebugger;\n",
            false,
            &Rules::default(),
        );
        let finding = &linted.findings[0];
        assert_eq!(finding.rule, "eslint/no-debugger");
        assert_eq!(
            finding.to_string(),
            format!(
                "src/app.tsx:3: error: eslint/no-debugger: {}",
                finding.message
            )
        );
    }

    #[test]
    fn a_fix_rewrites_and_is_idempotent() {
        let messy = "import {h, Fragment} from 'preact'\nimport { unused } from './x'\n\
                     export const A = () => <><p class='x'>hi</p><i /></>\n";
        let fixed = lint(messy, true, &Rules::default()).text.unwrap();
        assert_eq!(
            fixed,
            "import { h, Fragment } from \"preact\";\n\
             export const A = () => (\n  <>\n    <p class=\"x\">hi</p>\n    <i />\n  </>\n);\n"
        );
        let again = lint(&fixed, true, &Rules::default());
        assert_eq!(again.text, None);
        assert!(again.findings.is_empty(), "{:?}", again.findings);
    }

    #[test]
    fn a_rule_turned_off_does_not_run_and_a_category_turns_rules_on() {
        let text = "debugger;\n";
        assert_eq!(lint(text, false, &Rules::default()).findings.len(), 1);
        let off = config(&[], &[("eslint/no-debugger", json!("off"))]);
        assert_eq!(lint(text, false, &Rules::of(&off).unwrap()).findings, []);
        let notice = config(&[], &[("eslint/no-debugger", json!("notice"))]);
        assert!(lint(text, false, &Rules::of(&notice).unwrap()).findings[0].notice);
        // `no-magic-numbers` is in Oxlint's `style` and not in the preset: on with the category.
        let magic = "export const f = (x: number) => x * 37;\n";
        let has = |r: &Rules, rule: &str| {
            lint(magic, false, r)
                .findings
                .iter()
                .any(|f| f.rule == rule)
        };
        assert!(!has(&Rules::default(), "eslint/no-magic-numbers"));
        let style = config(&[("style", Level::Error)], &[]);
        assert!(has(&Rules::of(&style).unwrap(), "eslint/no-magic-numbers"));
    }

    #[test]
    fn a_rule_takes_its_options() {
        let text = "export const f = (a: number, b: number, c: number) => a + b + c;\n";
        let rule = "eslint/max-params";
        let count = |max: u32| {
            let rules = config(&[], &[(rule, json!(["error", { "max": max }]))]);
            lint(text, false, &Rules::of(&rules).unwrap())
                .findings
                .iter()
                .filter(|f| f.rule == rule)
                .count()
        };
        assert_eq!((count(2), count(3)), (1, 0));
    }

    #[test]
    fn an_unknown_rule_or_category_is_reported() {
        let error = |config: &Config| Rules::of(config).unwrap_err();
        let old_name = error(&config(&[], &[("suspicious/noDebugger", json!("off"))]));
        assert_eq!(old_name.name, "suspicious/noDebugger");
        assert!(
            old_name.message.starts_with(
                "rules: expected an Oxlint rule by the name a finding gives it \
                 (`eslint/no-debugger`), got \"suspicious/noDebugger\"; the plugin is one of "
            ),
            "{old_name}"
        );
        assert!(
            error(&config(&[], &[("eslint/no-vra", json!("off"))]))
                .message
                .contains("got \"eslint/no-vra\"")
        );
        assert_eq!(
            error(&config(&[], &[("eslint/no-debugger", json!("warn"))])).message,
            "rules: \"eslint/no-debugger\": expected error, notice or off, got \"warn\""
        );
        assert!(
            error(&config(
                &[],
                &[("typescript/no-floating-promises", json!("error"))]
            ))
            .message
            .contains("needs type information")
        );
        assert_eq!(
            error(&config(&[("stlye", Level::Error)], &[])).message,
            "categories: expected one of correctness, suspicious, pedantic, perf, style \
             or restriction, got \"stlye\""
        );
        let groups = Config {
            groups: Some(json!({ "style": "error" })),
            ..Config::default()
        };
        assert!(error(&groups).message.starts_with("groups: no such key"));
    }

    #[test]
    fn a_syntax_error_is_a_parse_finding() {
        let linted = lint("export const a = ;\n", true, &Rules::default());
        assert_eq!(linted.text, None);
        assert_eq!(linted.findings[0].rule, "parse");
        assert_eq!(linted.findings[0].line, 1);
    }
}
