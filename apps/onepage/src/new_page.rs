//! `onepage new`: a page folder to start from, which builds on the first try.

use std::path::Path;

use anyhow::{Context, Result, ensure};

const CONFIG: &str = "# onepage.toml: every key is optional. See the README for the full list.
title = \"@TITLE@\"
";

const SHELL: &str = "<!doctype html>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>@TITLE@</title>
<!--css-->
</head>
<body>
<div id=\"root\"></div>
<!--js-->
</body>
</html>
";

const APP: &str = "import { h, render } from \"preact\";
import { useState } from \"preact/hooks\";

function App() {
  const [count, setCount] = useState(0);
  return (
    <main class=\"mx-auto max-w-md p-8 font-sans\">
      <h1 class=\"text-2xl font-bold\">@TITLE@</h1>
      <button class=\"mt-4 rounded bg-black px-4 py-2 text-white\" onClick={() => setCount(count + 1)}>
        Clicked {count} times
      </button>
    </main>
  );
}

render(<App />, document.getElementById(\"root\")!);
";

/// Make the folder `dir` with `onepage.toml`, `index.html` and `src/app.tsx`. Returns what
/// to run next.
pub fn run(dir: &Path) -> Result<String> {
    ensure!(!dir.exists(), "{} exists already", dir.display());
    let title = dir
        .file_name()
        .map_or_else(|| "page".into(), |n| n.to_string_lossy());
    std::fs::create_dir_all(dir.join("src"))
        .with_context(|| format!("creating {}", dir.display()))?;
    for (file, text) in [
        ("onepage.toml", CONFIG),
        ("index.html", SHELL),
        ("src/app.tsx", APP),
    ] {
        std::fs::write(dir.join(file), text.replace("@TITLE@", &title))?;
    }
    Ok(format!(
        "made {0}; next: onepage dev {0} --port 8000",
        dir.display()
    ))
}
