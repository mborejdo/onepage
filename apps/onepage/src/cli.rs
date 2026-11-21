//! The command line definition: `Cli`, `Command` and the `--help` texts.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// What the top of `--help` says.
const ABOUT: &str = "A Preact TSX page in, one self-contained HTML file out

swc bundles the module graph, encre-css writes the utilities the page uses, and a strict
Content-Security-Policy pins the one inline script by its hash. No Node, no npm.";

/// The end of the root `--help`: examples and exit codes.
const ROOT_FOOTER: &str = "\
Examples:
  onepage new hello                 make hello/onepage.toml, index.html and src/app.tsx
  onepage build hello               write hello/dist/index.html, minified
  onepage dev hello --port 8000     rebuild on save, serve on http://127.0.0.1:8000/

Exit codes:
  0  done
  1  the page has a mistake (one `error: file:line: message` line each on stderr)
  2  onepage could not do what you asked (the reason is the one line on stderr)";

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
    /// Build the page into one minified HTML file.
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
