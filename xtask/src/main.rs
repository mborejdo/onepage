//! onepage's task runner: `cargo xtask assets [NAME] [--check]`.
//!
//! Rebuilds the vendored files from the pins in `assets.toml`: Preact's UMD builds, the Tabler
//! glyphs and the panelkit fonts. Downloads go through `curl` and must match their sha256; the
//! fonts are subset by fonttools through `uvx`. Dev tooling only: onepage itself starts nothing.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a task runner reports what it writes"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The tasks.
#[derive(Parser)]
#[command(name = "xtask", about = "onepage's tasks: cargo xtask <task>")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Rebuild the vendored files from the pins in assets.toml.
    Assets {
        /// Only this one: preact, tabler or a font's name.
        name: Option<String>,
        /// Compare the files with the hashes in `[out]`, offline; write nothing.
        #[arg(long)]
        check: bool,
    },
}

/// `assets.toml`.
#[derive(Deserialize)]
struct Manifest {
    fonttools: Fonttools,
    preact: Source,
    tabler: Tabler,
    fonts: Fonts,
    /// Every file written, by its path from the root, to its sha256.
    out: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Fonttools {
    version: String,
    brotli: String,
    uharfbuzz: String,
}

/// A pinned download; `{version}` in the URL is the version.
#[derive(Deserialize)]
struct Source {
    version: String,
    url: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Tabler {
    #[serde(flatten)]
    source: Source,
    icons: Vec<String>,
}

#[derive(Deserialize)]
struct Fonts {
    wght: String,
    unicodes: String,
    font: Vec<Font>,
}

#[derive(Deserialize)]
struct Font {
    name: String,
    #[serde(flatten)]
    source: Source,
    /// The variable font in the zip.
    file: String,
    /// The licence in the zip, or a URL pinned by `license_sha256`.
    license: String,
    license_sha256: Option<String>,
    features: String,
    epoch: u64,
}

/// Files to write: path from the root to contents.
type Files = BTreeMap<String, Vec<u8>>;

/// `tabler-nodes-outline.json`: each icon's elements as `[tag, attributes]`.
type Nodes = BTreeMap<String, Vec<(String, BTreeMap<String, String>)>>;

/// The workspace root: the folder above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// `url` through curl, refused unless it hashes to `sha256`.
fn fetch(url: &str, sha256_pin: &str) -> Result<Vec<u8>> {
    println!("fetch {url}");
    let out = Command::new("curl")
        .args(["-sSfL", url])
        .output()
        .context("start curl")?;
    ensure!(
        out.status.success(),
        "curl {url}: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    let got = sha256(&out.stdout);
    ensure!(
        got == sha256_pin,
        "{url}: sha256 is {got}, assets.toml pins {sha256_pin}"
    );
    Ok(out.stdout)
}

impl Source {
    fn fetch(&self) -> Result<Vec<u8>> {
        fetch(&self.url.replace("{version}", &self.version), &self.sha256)
    }
}

/// The files at `paths` in a gzipped tarball.
fn untar<const N: usize>(tgz: &[u8], paths: [&str; N]) -> Result<[Vec<u8>; N]> {
    let mut found = [const { None }; N];
    for entry in tar::Archive::new(flate2::read::GzDecoder::new(tgz)).entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();
        if let Some(i) = paths.iter().position(|p| *p == path) {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            found[i] = Some(bytes);
        }
    }
    for (file, path) in found.iter().zip(paths) {
        ensure!(file.is_some(), "{path}: not in the tarball");
    }
    Ok(found.map(Option::unwrap_or_default))
}

/// The file at `path` in a zip.
fn unzip(zip: &[u8], path: &str) -> Result<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip))?;
    let mut bytes = Vec::new();
    archive
        .by_name(path)
        .with_context(|| format!("{path}: not in the zip"))?
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// A UMD build without its last line, the `sourceMappingURL` comment.
fn without_source_map(js: &[u8]) -> Result<Vec<u8>> {
    let js = std::str::from_utf8(js)?;
    let (body, last) = js
        .trim_end()
        .rsplit_once('\n')
        .context("a one-line build")?;
    ensure!(
        last.starts_with("//# sourceMappingURL="),
        "no sourceMappingURL line at the end"
    );
    Ok(format!("{body}\n").into_bytes())
}

fn preact(pin: &Source) -> Result<Files> {
    let [umd, hooks, license] = untar(
        &pin.fetch()?,
        [
            "package/dist/preact.umd.js",
            "package/hooks/dist/hooks.umd.js",
            "package/LICENSE",
        ],
    )?;
    let mut notice = format!(
        "preact.umd.js and hooks.umd.js are Preact {} (dist/preact.umd.js and\n\
         hooks/dist/hooks.umd.js from the npm package), with the\n\
         sourceMappingURL comment removed.\n\n",
        pin.version
    )
    .into_bytes();
    notice.extend(license);
    Ok(Files::from([
        (
            "packages/pagekit/assets/preact.umd.js".into(),
            without_source_map(&umd)?,
        ),
        (
            "packages/pagekit/assets/hooks.umd.js".into(),
            without_source_map(&hooks)?,
        ),
        ("packages/pagekit/assets/LICENSE-preact".into(), notice),
    ]))
}

/// `alert-triangle` to `alertTriangle`.
fn camel(name: &str) -> String {
    let mut out = String::new();
    for (i, part) in name.split('-').enumerate() {
        let mut chars = part.chars();
        if i > 0
            && let Some(first) = chars.next()
        {
            out.extend(first.to_uppercase());
        }
        out.push_str(chars.as_str());
    }
    out
}

fn tabler(pin: &Tabler) -> Result<Files> {
    let [nodes, license] = untar(
        &pin.source.fetch()?,
        ["package/tabler-nodes-outline.json", "package/LICENSE"],
    )?;
    let nodes: Nodes = serde_json::from_slice(&nodes)?;
    let mut icons = pin.icons.clone();
    icons.sort();
    icons.dedup();
    let mut ts = format!(
        "// The glyphs: Tabler Icons {}, outline style (MIT, https://tabler.io/icons, see\n\
         // LICENSE-tabler), written by `cargo xtask assets` from the `icons` in assets.toml.\n\
         // One export per icon, Tabler's name in camel case with `Icon` behind it (`alert-triangle` is\n\
         // `alertTriangleIcon`, `player-play` is `playerPlayIcon`), each its SVG paths on a 24 x 24\n\
         // grid. `Icon` draws them with a 2px round stroke in the current colour. Imported by name, so a\n\
         // page carries only the glyphs it names; `GLYPHS` (every one, by Tabler's name) is for a\n\
         // catalogue and pulls in the lot.\n\n\
         import type {{ Glyph }} from \"./icon\";\n\n",
        pin.source.version
    );
    let mut table = String::new();
    for name in &icons {
        let node = nodes
            .get(name)
            .with_context(|| format!("{name}: not in Tabler {}", pin.source.version))?;
        let mut paths = Vec::new();
        for (tag, attrs) in node {
            let d = attrs.get("d").filter(|_| tag == "path" && attrs.len() == 1);
            let d = d.with_context(|| format!("{name}: a {tag} that is not a bare path"))?;
            ensure!(!d.contains(['"', '\\']), "{name}: a quote in a path");
            paths.push(format!("{{ d: \"{d}\" }}"));
        }
        let ident = format!("{}Icon", camel(name));
        writeln!(ts, "export const {ident}: Glyph = [{}];", paths.join(", "))?;
        writeln!(table, "  \"{name}\": {ident},")?;
    }
    write!(
        ts,
        "\n/** Every glyph by its Tabler name, for a catalogue: a page that imports this carries \
         them all. */\nexport const GLYPHS: Record<string, Glyph> = {{\n{table}}};\n"
    )?;
    Ok(Files::from([
        ("packages/panelkit/src/glyphs.ts".into(), ts.into_bytes()),
        ("packages/panelkit/LICENSE-tabler".into(), license),
    ]))
}

/// `wght` limited to the range, then a woff2 subset, by the pinned fonttools.
fn font(m: &Manifest, font: &Font) -> Result<Files> {
    let zip = font.source.fetch()?;
    let license = match &font.license_sha256 {
        Some(pin) => fetch(
            &font.license.replace("{version}", &font.source.version),
            pin,
        )?,
        None => unzip(&zip, &font.license)?,
    };
    let dir = std::env::temp_dir().join("onepage-assets");
    fs::create_dir_all(&dir)?;
    let (vf, ttf, woff2) = (
        dir.join(format!("{}-vf.ttf", font.name)),
        dir.join(format!("{}.ttf", font.name)),
        dir.join(format!("{}.woff2", font.name)),
    );
    fs::write(&vf, unzip(&zip, &font.file)?)?;
    let uvx = |tool: &str| {
        let mut cmd = Command::new("uvx");
        cmd.arg("--from")
            .arg(format!("fonttools=={}", m.fonttools.version))
            .arg("--with")
            .arg(format!("brotli=={}", m.fonttools.brotli))
            .arg("--with")
            .arg(format!("uharfbuzz=={}", m.fonttools.uharfbuzz))
            .arg(tool)
            .env("SOURCE_DATE_EPOCH", font.epoch.to_string());
        cmd
    };
    run(uvx("fonttools")
        .args(["varLib.instancer".as_ref(), vf.as_os_str()])
        .arg(format!("wght={}", m.fonts.wght))
        .arg("-q")
        .arg("-o")
        .arg(&ttf))?;
    run(uvx("pyftsubset")
        .arg(&ttf)
        .arg(format!("--unicodes={}", m.fonts.unicodes))
        .arg(format!("--layout-features={}", font.features))
        .arg("--flavor=woff2")
        .arg(format!("--output-file={}", woff2.display())))?;
    Ok(Files::from([
        (
            format!("packages/panelkit/fonts/{}.woff2", font.name),
            fs::read(&woff2)?,
        ),
        (format!("packages/panelkit/LICENSE-{}", font.name), license),
    ]))
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| {
        format!(
            "start {} (uv, https://docs.astral.sh/uv)",
            cmd.get_program().display()
        )
    })?;
    ensure!(status.success(), "{cmd:?}: {status}");
    Ok(())
}

fn assets(name: Option<&str>, check: bool) -> Result<()> {
    let root = root();
    let path = root.join("assets.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let m: Manifest = toml::from_str(&text).context("assets.toml")?;
    if check {
        let mut stale = Vec::new();
        for (file, pin) in &m.out {
            let bytes = fs::read(root.join(file)).with_context(|| format!("read {file}"))?;
            if sha256(&bytes) != *pin {
                stale.push(file.as_str());
            }
        }
        ensure!(!m.out.is_empty(), "assets.toml: [out] is empty");
        if !stale.is_empty() {
            bail!(
                "not what assets.toml pins, run `cargo xtask assets`: {}",
                stale.join(", ")
            );
        }
        println!("assets: {} files match assets.toml", m.out.len());
        return Ok(());
    }
    let names = ["preact", "tabler"]
        .into_iter()
        .chain(m.fonts.font.iter().map(|f| f.name.as_str()));
    if let Some(name) = name
        && !names.clone().any(|n| n == name)
    {
        bail!(
            "no asset {name}; one of: {}",
            names.collect::<Vec<_>>().join(", ")
        );
    }
    let wanted = |n: &str| name.is_none_or(|w| w == n);
    let mut files = Files::new();
    if wanted("preact") {
        files.extend(preact(&m.preact)?);
    }
    if wanted("tabler") {
        files.extend(tabler(&m.tabler)?);
    }
    for f in m.fonts.font.iter().filter(|f| wanted(&f.name)) {
        files.extend(font(&m, f)?);
    }
    let mut out = m.out.clone();
    for (file, bytes) in &files {
        let hash = sha256(bytes);
        let state = if out.get(file) == Some(&hash) {
            "same"
        } else {
            "changed"
        };
        println!("{state:>7} {file}");
        fs::write(root.join(file), bytes).with_context(|| format!("write {file}"))?;
        out.insert(file.clone(), hash);
    }
    let (pins, _) = text
        .split_once("\n[out]\n")
        .context("assets.toml: [out] must come last")?;
    let mut new = format!("{pins}\n[out]\n");
    for (file, hash) in &out {
        writeln!(new, "\"{file}\" = \"{hash}\"")?;
    }
    fs::write(&path, new).context("write assets.toml")
}

fn main() -> ExitCode {
    let result = match Cli::parse().task {
        Task::Assets { name, check } => assets(name.as_deref(), check),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            ExitCode::FAILURE
        },
    }
}
