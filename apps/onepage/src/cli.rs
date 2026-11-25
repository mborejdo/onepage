//! The command line definition: `Cli`, `Command` and the `--help` texts.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// What the top of `--help` says.
const ABOUT: &str = "A Preact TSX page in, one self-contained HTML file out

Bundled by rolldown, styled by encre-css, locked by a hash-pinned CSP. No Node, no npm.";

/// The end of the root `--help`: examples and exit codes.
const ROOT_FOOTER: &str = "\
Examples:
  onepage new hello                 make hello/onepage.toml, index.html, src/app.tsx
  onepage build hello               write hello/dist/index.html, minified
  onepage lint hello --fix          the safe fixes and the formatter, written back
  onepage dev hello --port 8000     rebuild on save, serve on http://127.0.0.1:8000/

Exit codes:
  0  done
  1  the page has a mistake (one `file:line: error: ...` line each)
  2  onepage could not do what you asked (the reason on stderr)";

/// The end of `onepage build --help`.
const BUILD_FOOTER: &str = "\
Examples:
  onepage build hello                         write hello/dist/index.html, minified
  onepage build hello --out site/index.html   write there instead

Exit codes:
  0  written (its path and size on stdout)
  1  the page has a mistake (one `file:line: error: message` line each on stderr)
  2  onepage could not do what you asked (the reason on stderr)";

/// The end of `onepage dev --help`.
const DEV_FOOTER: &str = "\
Examples:
  onepage dev hello                 rebuild hello/dist/index.html on save, unminified
  onepage dev hello --port 8000     and serve it on http://127.0.0.1:8000/

It runs until stopped; a failed rebuild prints its reason on stderr and waits for the next save.

Exit codes:
  2  the config can't be read or the port can't be bound (the reason on stderr)";

/// The end of `onepage lint --help`.
const LINT_FOOTER: &str = "\
Examples:
  onepage lint hello                one `file:line: error: rule: message` line per finding, on stdout
  onepage lint hello --fix          the safe fixes and the formatter, written back
                                    (a `<path>  <N> KB` line per file rewritten)

The rules are onepage's preset of Oxlint rules. A `[lint]` table in onepage.toml moves them by
Oxlint category and by the name a finding gives a rule, to error, notice or off:
categories = { style = \"error\" }, rules = { \"eslint/no-console\" = \"off\" }.

Exit codes:
  0  no errors (notes only)
  1  at least one error
  2  onepage could not do what you asked (the reason on stderr)";

/// The end of `onepage new --help`.
const NEW_FOOTER: &str = "\
Examples:
  onepage new hello                 make hello/onepage.toml, index.html, src/app.tsx

Exit codes:
  0  made
  2  the folder exists already or can't be written (the reason on stderr)";

/// The end of `onepage test --help`.
#[cfg(feature = "test")]
const TEST_FOOTER: &str = "\
Examples:
  onepage test hello                run hello/src/**/*.test.ts, one line per test

Exit codes:
  0  every test passed
  1  at least one test failed
  2  onepage could not do what you asked (the reason on stderr)";

/// The end of `onepage types --help`.
#[cfg(feature = "types")]
const TYPES_FOOTER: &str = "\
Examples:
  onepage types hello               one `file:line: error|note: code: message` line per finding

The program is the page's own sources, its [imports] and vendored typings and Preact's; the
codes are tsc's (TS2322) and the type-aware rules' (typescript/no-floating-promises). `lint` and
`build` run the same checks. A `[types]` table in onepage.toml is the ratchet: per code, how many
findings the page may have (TS2322 = 3); a code over its count is an error, and without the
table every finding is a note.

Exit codes:
  0  no code over its [types] count
  1  at least one
  2  onepage could not do what you asked (the reason on stderr)";

/// The `onepage` command line.
#[derive(Debug, Parser)]
#[command(name = "onepage", version, about = ABOUT, after_help = ROOT_FOOTER)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Fix and format the page's sources, then build it into one minified HTML file.
    #[command(after_help = BUILD_FOOTER)]
    Build {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Write here instead of `out` from onepage.toml (--out site/index.html).
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    /// Rebuild on every change, unminified; serve the result with `--port`.
    #[command(after_help = DEV_FOOTER)]
    Dev {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Serve the output folder on 127.0.0.1 at this port (--port 8000).
        #[arg(long, value_name = "PORT")]
        port: Option<u16>,
    },
    /// Lint the page's sources with Oxc; a file the formatter would change is a finding.
    #[command(after_help = LINT_FOOTER)]
    Lint {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Apply the safe fixes and the formatter, in place.
        #[arg(long)]
        fix: bool,
    },
    /// Make a new page folder that builds on the first try.
    #[command(after_help = NEW_FOOTER)]
    New {
        /// The folder to create.
        name: PathBuf,
    },
    /// Type-check the page's sources with tsrs (the TypeScript 7 checker in Rust).
    #[cfg(feature = "types")]
    #[command(after_help = TYPES_FOOTER)]
    Types {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Run the page's `src/**/*.test.ts` files in an embedded JavaScript engine.
    #[cfg(feature = "test")]
    #[command(after_help = TEST_FOOTER)]
    Test {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn help_of(command: &clap::Command) -> String {
        command.clone().render_long_help().to_string()
    }

    /// The root and every subcommand but `help`.
    fn commands() -> Vec<clap::Command> {
        let root = Cli::command();
        let mut commands = vec![root.clone()];
        commands.extend(
            root.get_subcommands()
                .filter(|c| c.get_name() != "help")
                .cloned(),
        );
        commands
    }

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_help_has_examples_and_exit_codes() {
        for command in commands() {
            let help = help_of(&command);
            assert!(help.contains("Examples:"), "{}", command.get_name());
            assert!(help.contains("Exit codes:"), "{}", command.get_name());
        }
    }

    #[test]
    fn every_help_is_keyboard_characters() {
        for command in commands() {
            assert!(help_of(&command).is_ascii(), "{}", command.get_name());
        }
    }

    #[test]
    fn every_flag_has_an_example() {
        for command in commands() {
            let help = help_of(&command);
            for arg in command.get_arguments() {
                let Some(long) = arg.get_long().filter(|l| !matches!(*l, "help" | "version"))
                else {
                    continue;
                };
                let flag = format!("--{long}");
                let in_flag_help = arg
                    .get_help()
                    .is_some_and(|h| h.to_string().contains(&flag));
                let in_footer = help
                    .lines()
                    .filter(|l| l.trim_start().starts_with("onepage "))
                    .any(|l| l.contains(&flag));
                assert!(in_flag_help || in_footer, "{flag} has no example");
            }
        }
    }
}
