//! Runs the equation search script's own tests (`assets/mazer-search.test.mjs`)
//! under Node, and checks the script against real pages from the CLI: the
//! inline script is valid JavaScript (`node --check`), the page has one
//! source per `<math>` element, and every `show` form of the note, pasted
//! into the box exactly as it is typed in the note (with `(show`, its tag,
//! comments and line breaks), finds the equation it rendered.
//!
//! The pages are `examples/docs.md`, `examples/showcase.zr` and a note written here with the reader
//! shorthands, a tag, an alias, a macro, a `let`, strings with escapes and a
//! multi-line form with a comment.
//!
//! Skipped when no `node` is on PATH.

use std::fmt::Write as _;
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

/// A note for the search: every shape of source the reader has.
const NOTE: &str = r#"# Search (show (frac 1 2)) test

(alias Field (bb F))
(macro (half v) (frac v 2))
(let zq 5)

The mass:

(show (= (integral (funcapp rho x) x)
         (integral (jux m (funcapp delta (- x a))) x)   ; a comment
         m))

Inline (show (funcapp rho x)) and (show (+ -1 -x (- y) 3.0e-2)) here.

(show (tag mass) (= (funcapp Delta x) (funcapp delta x)))

(show (tag q1) ; tagged, over two lines
      (= q 1))

Field: (show (in x Field)) and (show (half zq)).

(show (sum k 'k 1 n))

(show (= 'zq zq))

(show (text "say \"hi\" ; not a comment \\ \n & <b>"))

(show a b c)

(show (+ (sqrt x)(sqrt y)))

- in a list (show (funcapp phi t))

> quoted (show (funcapp chi u))
"#;

/// `text` as a JSON string.
fn json(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The `show` and `eval` forms of `src` exactly as typed, in document order,
/// as a JSON array, read with mazer's own markdown parser and reader (code
/// in fenced blocks and inline code is not a form).
fn forms(src: &str) -> String {
    let analysis = mazer_doc::analyze(src, "note.zr");
    let forms: Vec<String> = mazer_doc::code_forms(&analysis)
        .into_iter()
        .map(|(form, _)| form)
        .filter(|form| form.is_form("show") || form.is_form("eval"))
        .map(|form| json(analysis.sources.text(form.span)))
        .collect();
    assert_ne!(forms.len(), 0, "a note with no forms");
    format!("[{}]", forms.join(","))
}

/// Compiles `note` (a file) to a full page in `dir`, and writes its forms
/// next to it. Returns the page and the forms file.
fn page_and_forms(dir: &Path, note: &Path, name: &str) -> (PathBuf, PathBuf) {
    let page = dir.join(format!("{name}.html"));
    let compiled = Command::new(env!("CARGO_BIN_EXE_mazer"))
        .arg(note)
        .arg("-o")
        .arg(&page)
        .arg("--strict")
        .env("NO_COLOR", "1")
        .output()
        .expect("run mazer");
    assert!(compiled.status.success(), "{name}: {}", show(&compiled));
    assert!(page.is_file(), "no page written: {}", show(&compiled));
    let forms = dir.join(format!("{name}.forms.json"));
    fs::write(&forms, self::forms(&fs::read_to_string(note).expect("read the note"))).expect("write the forms");
    (page, forms)
}

#[test]
fn search_script_tests_pass_under_node() {
    let Some(node) = node() else {
        eprintln!("skipping: no node on PATH");
        return;
    };
    let dir = Scratch(std::env::temp_dir().join(format!("mazer-search-js-{}", std::process::id())));
    fs::create_dir_all(&dir.0).expect("create temp dir");

    let (docs, docs_forms) = page_and_forms(&dir.0, &repo().join("examples/docs.md"), "docs");
    let (showcase, showcase_forms) = page_and_forms(&dir.0, &repo().join("examples/showcase.zr"), "showcase");
    let scratch_note = dir.0.join("note.zr");
    fs::write(&scratch_note, NOTE).expect("write the note");
    let (scratch, scratch_forms) = page_and_forms(&dir.0, &scratch_note, "note");
    let html = fs::read_to_string(&scratch).expect("read the page");
    // The sources are as typed, not as the reader made them.
    for typed in ["(+ -1 -x (- y) 3.0e-2)", "(sum k &#39;k 1 n)", r"\&quot;hi\&quot; ; not a comment \\ \n &amp;"] {
        assert!(html.contains(typed), "{typed} not in the page");
    }

    // The inline script, as the browser would read it, parses.
    let html = fs::read_to_string(&docs).expect("read the page");
    let start = html.rfind("<script>").expect("a script") + "<script>".len();
    let end = start + html[start..].find("</script>").expect("its end");
    let script = dir.0.join("inline.js");
    fs::write(&script, &html[start..end]).expect("write the script");
    let check = Command::new(&node).arg("--check").arg(&script).output().expect("node runs");
    assert!(check.status.success(), "node --check: {}", show(&check));

    let tests = repo().join("assets/mazer-search.test.mjs");
    let run = Command::new(&node)
        .arg(&tests)
        .args([&docs, &docs_forms, &showcase, &showcase_forms, &scratch, &scratch_forms])
        .output()
        .expect("node runs");
    let text = show(&run);
    eprintln!("{text}");
    assert!(run.status.success(), "{text}");
    assert!(text.contains("tests passed"), "{text}");
    // All three pages were checked, each with all of its forms.
    assert_eq!(text.matches("each found by its form as typed").count(), 3, "{text}");
}
