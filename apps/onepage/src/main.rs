//! `onepage`: build a Preact TSX page into one self-contained HTML file.
//!
//! ```sh
//! onepage new hello
//! onepage build hello
//! onepage lint hello --fix
//! onepage dev hello --port 8000
//! ```

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the binary prints what each command did, and the failure on stderr"
)]

mod cli;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn run(cli: Cli) -> anyhow::Result<u8> {
    match cli.command {
        Command::Build { dir, out } => {
            let (out, lint) = onepage::build(&dir, out.as_deref(), true)?;
            eprint!("{lint}");
            for file in &lint.formatted {
                println!("{}", onepage::written(&dir.join(file)));
            }
            println!("{}", onepage::written(&out));
        },
        Command::Dev { dir, port } => onepage::dev::run(&dir, port)?,
        Command::Lint { dir, fix } => {
            let lint = onepage::lint(&dir, fix)?;
            print!("{lint}");
            for file in &lint.formatted {
                println!("{}", onepage::written(&dir.join(file)));
            }
            println!("{} errors, 0 warnings", lint.errors());
            return Ok(u8::from(lint.errors() > 0));
        },
        Command::New { name } => println!("{}", onepage::new_page::run(&name)?),
        #[cfg(feature = "types")]
        Command::Types { dir } => {
            let types = onepage::types(&dir)?;
            print!("{types}");
            println!("{} errors, 0 warnings", types.errors());
            return Ok(u8::from(types.errors() > 0));
        },
        #[cfg(feature = "test")]
        Command::Test { dir } => {
            let page = onepage::config::Config::load(&dir)?.page(&dir, false);
            let outcomes = pagekit::test::run(&page)?;
            for outcome in &outcomes {
                println!("{}", outcome.file.display());
                for line in &outcome.lines {
                    println!("  {line}");
                }
            }
            return Ok(u8::from(
                !outcomes.iter().all(pagekit::test::Outcome::passed),
            ));
        },
    }
    Ok(0)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            let (line, code) = onepage::report(&e);
            eprintln!("{line}");
            ExitCode::from(code)
        },
    }
}
