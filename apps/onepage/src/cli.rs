//! The command line definition: `Cli`, `Command` and the `--help` texts.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// What the top of `--help` says.
const ABOUT: &str = "A Preact TSX page in, one self-contained HTML file out

Bundled by swc, styled by encre-css, locked by a hash-pinned CSP. No Node, no npm.";

/// The end of the root `--help`: examples and exit codes.
const ROOT_FOOTER: &str = "\
Examples:
  onepage new hello                 make hello/onepage.toml, index.html, src/app.tsx
  onepage build hello               write hello/dist/index.html, minified
  onepage lint hello --fix          Biome's safe fixes and formatter, written back
  onepage dev hello --port 8000     rebuild on save, serve on http://127.0.0.1:8000/

Exit codes:
  0  done
  1  the page has a mistake (one `error: file:line: message` line each on stderr)
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
    Build {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Write here instead of `out` from onepage.toml.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Rebuild on every change, unminified; serve the result with `--port`.
    Dev {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Serve the output folder on 127.0.0.1 at this port.
        #[arg(long)]
        port: Option<u16>,
    },
    /// Lint the page's sources with Biome; a file the formatter would change is a finding.
    Lint {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Apply the safe fixes and the formatter, in place.
        #[arg(long)]
        fix: bool,
    },
    /// Make a new page folder that builds on the first try.
    New {
        /// The folder to create.
        name: PathBuf,
    },
    /// Run the page's `src/**/*.test.ts` files in an embedded JavaScript engine.
    #[cfg(feature = "test")]
    Test {
        /// The page's folder.
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
}
