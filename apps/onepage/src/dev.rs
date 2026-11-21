//! `onepage dev`: rebuild on every change, and serve the output folder on 127.0.0.1.
//!
//! Changes are found by polling file modification times, so it needs no file watcher. The
//! server is a minimal static one over `std::net`: GET only, one thread per connection.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the dev loop reports each rebuild and its failure"
)]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use crate::config::Config;

/// Watch `dir` and rebuild it unminified on every change; with `port`, serve the result.
pub fn run(dir: &Path, port: Option<u16>) -> Result<()> {
    let out = dir.join(Config::load(dir)?.out);
    let folder = out.parent().unwrap_or(dir).to_path_buf();
    if let Some(port) = port {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .with_context(|| format!("binding 127.0.0.1:{port}"))?;
        let index = out.file_name().map(PathBuf::from).unwrap_or_default();
        let root = folder.clone();
        std::thread::spawn(move || serve(&listener, &root, &index));
        println!("serving http://127.0.0.1:{port}/");
    }
    let mut seen = None;
    loop {
        let now = snapshot(dir, &folder);
        if seen.as_ref() != Some(&now) {
            seen = Some(now);
            match crate::build(dir, None, false) {
                Ok(path) => println!("built {}", path.display()),
                Err(e) => eprintln!("{}", crate::report(&e).0),
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// Every file under `dir` with its modification time, leaving out `skip` and dot folders.
fn snapshot(dir: &Path, skip: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for path in std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
        {
            let hidden = path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'));
            if path == skip || hidden {
            } else if path.is_dir() {
                stack.push(path);
            } else if let Ok(time) = path.metadata().and_then(|m| m.modified()) {
                out.push((path, time));
            }
        }
    }
    out.sort();
    out
}

/// Answer GET requests with the files in `root`; `/` is `index`.
fn serve(listener: &TcpListener, root: &Path, index: &Path) {
    for stream in listener.incoming().flatten() {
        let (root, index) = (root.to_path_buf(), index.to_path_buf());
        std::thread::spawn(move || answer(stream, &root, &index));
    }
}

fn answer(mut stream: TcpStream, root: &Path, index: &Path) -> std::io::Result<()> {
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let path = line.split(' ').nth(1).unwrap_or("/");
    let path = path
        .split(['?', '#'])
        .next()
        .unwrap_or("/")
        .trim_start_matches('/');
    let file = if path.is_empty() {
        index
    } else {
        Path::new(path)
    };
    let body = (!path.contains(".."))
        .then(|| std::fs::read(root.join(file)).ok())
        .flatten();
    let (status, body) = body.map_or_else(
        || ("404 Not Found", b"not found".to_vec()),
        |b| ("200 OK", b),
    );
    let kind = match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("json" | "webmanifest") => "application/json",
        _ => "application/octet-stream",
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&body)
}
