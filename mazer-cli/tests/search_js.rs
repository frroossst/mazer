//! Runs the equation search script's own tests (`assets/mazer-search.test.mjs`)
//! under Node, and checks the script the CLI puts into a real page: it is
//! valid JavaScript (`node --check`), and every equation source on that page
//! finds itself.
//!
//! Skipped when no `node` is on PATH.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn node() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("node"))
        .find(|p| p.is_file())
        .filter(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn show(out: &Output) -> String {
    format!(
        "status {}\n{}{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A scratch directory for this test, removed on drop.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn search_script_tests_pass_under_node() {
    let Some(node) = node() else {
        eprintln!("skipping: no node on PATH");
        return;
    };
    let dir = Scratch(std::env::temp_dir().join(format!("mazer-search-js-{}", std::process::id())));
    fs::create_dir_all(&dir.0).expect("create temp dir");

    // A real page from the CLI, from the larger example note.
    let page = dir.0.join("docs.html");
    let compiled = Command::new(env!("CARGO_BIN_EXE_mazer"))
        .arg(repo().join("examples/docs.md"))
        .arg("-o")
        .arg(&page)
        .env("NO_COLOR", "1")
        .output()
        .expect("run mazer");
    assert!(page.is_file(), "no page written: {}", show(&compiled));
    let html = fs::read_to_string(&page).expect("read the page");

    // The inline script, as the browser would read it, parses.
    let start = html.rfind("<script>").expect("a script") + "<script>".len();
    let end = start + html[start..].find("</script>").expect("its end");
    let script = dir.0.join("inline.js");
    fs::write(&script, &html[start..end]).expect("write the script");
    let check = Command::new(&node).arg("--check").arg(&script).output().expect("node runs");
    assert!(check.status.success(), "node --check: {}", show(&check));

    let tests = repo().join("assets/mazer-search.test.mjs");
    let run = Command::new(&node).arg(&tests).arg(&page).output().expect("node runs");
    let text = show(&run);
    eprintln!("{text}");
    assert!(run.status.success(), "{text}");
    assert!(text.contains("tests passed"), "{text}");
}
