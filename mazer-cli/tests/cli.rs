//! Runs the built `mazer` binary on small temp files. Never opens a browser.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh scratch directory, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "mazer-cli-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let p = self.0.join(name);
        fs::write(&p, text).expect("write input");
        p
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const GOOD: &str = "# Title\n\nSome text (show (frac 1 2)).\n";
const BAD: &str = "# Title\n\n(show (frac 1))\n\nStill fine (show (+ a b)).\n";

fn mazer(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mazer"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run mazer")
}

fn mazer_str(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mazer"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run mazer")
}

/// Runs `mazer` with `input` piped to its standard input.
fn mazer_stdin(args: &[&str], input: &str) -> Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_mazer"))
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mazer");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("exit code")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn default_output_is_next_to_input() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(err(&o).is_empty(), "{}", err(&o));
    let html = fs::read_to_string(d.path("note.html")).expect("note.html written");
    assert!(html.starts_with("<!DOCTYPE html>"), "{html}");
    assert!(html.contains("<title>note</title>"), "{html}");
    assert!(html.contains("<mfrac>"), "{html}");
}

#[test]
fn output_flag_writes_there() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let target = d.path("elsewhere.html");
    let o = mazer(&[&input, Path::new("-o"), &target]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(target.exists());
    assert!(!d.path("note.html").exists());
}

#[test]
fn output_dash_is_stdout() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input, Path::new("-o"), Path::new("-")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("<mfrac>"), "{}", out(&o));
    assert!(!d.path("note.html").exists());
}

#[test]
fn fragment_has_no_wrapper() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input, Path::new("--fragment"), Path::new("-o"), Path::new("-")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let html = out(&o);
    assert!(html.contains("<mfrac>"), "{html}");
    assert!(!html.contains("<html"), "{html}");
    assert!(!html.contains("<body"), "{html}");
}

// ---- the equation search box ----

/// The search script, as the CLI embeds it.
const SEARCH: &str = include_str!("../../assets/mazer-search.js");

/// Checks that `html` is a full page with the search script exactly once,
/// as the last thing in the body, and that the page is still well-formed
/// around it.
fn assert_has_search(html: &str) {
    assert_eq!(html.matches(SEARCH).count(), 1, "the search script once");
    let tail = format!("<script>{SEARCH}</script></body></html>");
    assert!(html.ends_with(&tail), "the script sits just before </body>");
    assert_eq!(html.matches("</body>").count(), 1);
    assert_eq!(html.matches("<script").count(), html.matches("</script>").count());
    assert!(html.starts_with("<!DOCTYPE html>"));
}

#[test]
fn full_page_has_the_search_script() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let html = fs::read_to_string(d.path("note.html")).expect("note.html written");
    assert_has_search(&html);
    assert!(html.contains("<mfrac>"), "{html}");
}

#[test]
fn full_page_to_stdout_has_the_search_script() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input, Path::new("-o"), Path::new("-")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_has_search(&out(&o));
    let piped = mazer_stdin(&["-"], GOOD);
    assert_eq!(code(&piped), 0, "{}", err(&piped));
    assert_has_search(&out(&piped));
}

#[test]
fn fragment_has_no_search_script() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input, Path::new("--fragment"), Path::new("-o"), Path::new("-")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let html = out(&o);
    assert!(!html.contains("__mazerSearch"), "{html}");
    assert!(!html.contains("<script"), "{html}");
    let to_file = mazer(&[&input, Path::new("--fragment")]);
    assert_eq!(code(&to_file), 0, "{}", err(&to_file));
    let html = fs::read_to_string(d.path("note.html")).expect("note.html written");
    assert!(!html.contains("__mazerSearch"), "{html}");
}

#[test]
fn search_script_is_inlinable() {
    assert!(!SEARCH.to_ascii_lowercase().contains("</script"));
    assert!(std::str::from_utf8(include_bytes!("../../assets/mazer-search.js")).is_ok());
}

#[cfg(feature = "latex")]
#[test]
fn latex_full_page_has_the_search_script() {
    let d = Dir::new();
    let input = d.write("note.zr", "Inline (show (frac 1 2)).\n\n(show (sqrt x))\n");
    let o = mazer(&[&input, Path::new("--target"), Path::new("latex"), Path::new("-o"), Path::new("-")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let html = out(&o);
    assert!(html.contains("\\("), "{html}");
    assert_has_search(&html);
    let fragment = mazer(&[
        &input,
        Path::new("--target"),
        Path::new("latex"),
        Path::new("--fragment"),
        Path::new("-o"),
        Path::new("-"),
    ]);
    assert!(!out(&fragment).contains("__mazerSearch"));
}

#[test]
fn check_without_errors_is_silent_and_writes_nothing() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input, Path::new("--check")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(err(&o).is_empty(), "{}", err(&o));
    assert!(!d.path("note.html").exists());
}

#[test]
fn check_with_errors_exits_1() {
    let d = Dir::new();
    let input = d.write("note.zr", BAD);
    let o = mazer(&[&input, Path::new("--check")]);
    assert_eq!(code(&o), 1);
    assert!(!d.path("note.html").exists());
    assert!(err(&o).contains("error[arity]"), "{}", err(&o));
}

#[test]
fn check_reports_eval_without_evaluator() {
    let d = Dir::new();
    let input = d.write("note.zr", "(eval (+ 1 2))\n");
    let o = mazer(&[&input, Path::new("--check")]);
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("error[no-evaluator]"), "{}", err(&o));
}

#[test]
fn warnings_do_not_fail() {
    let d = Dir::new();
    let input = d.write("note.zr", "[x](javascript:alert(1))\n");
    let o = mazer(&[&input]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(err(&o).contains("warning[other]"), "{}", err(&o));
    assert!(d.path("note.html").exists());
}

#[test]
fn note_with_error_still_writes_html_and_exits_1() {
    let d = Dir::new();
    let input = d.write("note.zr", BAD);
    let o = mazer(&[&input]);
    assert_eq!(code(&o), 1);
    let html = fs::read_to_string(d.path("note.html")).expect("html still written");
    assert!(html.contains("<mfrac>") || html.contains("merror"), "{html}");
    // The rest of the note renders.
    assert!(html.contains("<mi>a</mi>"), "{html}");
}

#[test]
fn diagnostics_have_file_line_col_and_summary() {
    let d = Dir::new();
    let input = d.write("note.zr", BAD);
    let o = mazer(&[&input]);
    let e = err(&o);
    assert!(e.contains("error[arity]"), "{e}");
    assert!(e.contains("note.zr:3:7"), "{e}");
    assert!(e.contains("(show (frac 1))"), "{e}");
    assert!(e.contains("1 error\n"), "{e}");
    assert!(!e.contains('\u{1b}'), "no colour when not a terminal");
}

#[test]
fn unreadable_input_exits_2() {
    let d = Dir::new();
    let o = mazer(&[&d.path("missing.zr")]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("cannot read"), "{}", err(&o));
}

#[test]
fn unwritable_output_exits_2() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let bad = d.path("no-such-dir").join("x.html");
    let o = mazer(&[&input, Path::new("-o"), &bad]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("cannot write"), "{}", err(&o));
}

#[test]
fn refuses_to_overwrite_input() {
    let d = Dir::new();
    let input = d.write("note.html", GOOD);
    let o = mazer(&[&input]);
    assert_eq!(code(&o), 2);
    assert_eq!(fs::read_to_string(&input).expect("read"), GOOD);
}

#[test]
fn usage_errors_exit_2() {
    for args in [&[][..], &["--bogus"], &["a.zr", "b.zr"], &["--target", "docx", "a.zr"]] {
        let o = mazer_str(args);
        assert_eq!(code(&o), 2, "{args:?}");
        assert!(err(&o).contains("--help"), "{args:?}: {}", err(&o));
    }
}

#[test]
fn help_and_version() {
    let o = mazer_str(&["--help"]);
    assert_eq!(code(&o), 0);
    assert!(out(&o).contains("Usage:"), "{}", out(&o));
    let o = mazer_str(&["-V"]);
    assert_eq!(code(&o), 0);
    assert!(out(&o).starts_with("mazer 0.16"), "{}", out(&o));
}

#[test]
fn query_finds_keyword_and_display_name() {
    let o = mazer_str(&["query", "integral"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("(integral "), "{}", out(&o));
    let o = mazer_str(&["query", "tau"]);
    assert_eq!(code(&o), 0);
    assert!(out(&o).contains("tau \u{2192} \u{3c4}"), "{}", out(&o));
}

#[test]
fn old_query_flag_still_works() {
    let o = mazer_str(&["-q", "integral"]);
    assert_eq!(code(&o), 0);
    assert!(out(&o).contains("(integral "), "{}", out(&o));
}

#[test]
fn query_without_match_exits_1() {
    let o = mazer_str(&["query", "zzzzqqqq"]);
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("nothing matches"), "{}", err(&o));
}

#[test]
fn query_searches_note_macros() {
    let d = Dir::new();
    let note = d.write("n.zr", "(macro (zorp a) \"Zorps it.\" (frac a 2))\n");
    let o = Command::new(env!("CARGO_BIN_EXE_mazer"))
        .args(["query", "zorp", "--file"])
        .arg(&note)
        .output()
        .expect("run");
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("(zorp a)"), "{}", out(&o));
    assert!(out(&o).contains("n.zr:1:"), "{}", out(&o));
}

#[cfg(not(feature = "latex"))]
#[test]
fn latex_target_rejected_without_feature() {
    let d = Dir::new();
    let input = d.write("note.zr", GOOD);
    let o = mazer(&[&input, Path::new("--target"), Path::new("latex")]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("--features latex"), "{}", err(&o));
    assert!(!d.path("note.html").exists());
}

#[cfg(feature = "latex")]
#[test]
fn latex_target_produces_latex_math() {
    let d = Dir::new();
    let input = d.write("note.zr", "Inline (show (frac 1 2)).\n\n(show (sqrt x))\n");
    let o = mazer(&[
        &input,
        Path::new("--target"),
        Path::new("latex"),
        Path::new("-o"),
        Path::new("-"),
    ]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let html = out(&o);
    assert!(html.contains("\\("), "{html}");
    assert!(html.contains("\\["), "{html}");
}

// ---- the output may never be the input ----

/// Runs `mazer <input> -o <output>` in `dir` and checks the refusal.
fn assert_refused(dir: &Dir, input: &Path, output: &Path) {
    let before = fs::read_to_string(dir.0.join(input)).expect("read input");
    let o = Command::new(env!("CARGO_BIN_EXE_mazer"))
        .current_dir(&dir.0)
        .arg(input)
        .arg("-o")
        .arg(output)
        .env("NO_COLOR", "1")
        .output()
        .expect("run mazer");
    assert_eq!(code(&o), 2, "{}: {}", output.display(), err(&o));
    assert!(err(&o).contains("refusing to overwrite"), "{}", err(&o));
    assert_eq!(fs::read_to_string(dir.0.join(input)).expect("read input"), before, "{}", output.display());
}

#[test]
fn refuses_dot_slash_spelling_of_the_input() {
    let d = Dir::new();
    let input = d.write("x.zr", GOOD);
    assert_refused(&d, Path::new("./x.zr"), Path::new("./x.zr"));
    assert_refused(&d, &input, Path::new("./x.zr"));
}

#[test]
fn refuses_absolute_path_to_the_input() {
    let d = Dir::new();
    let input = d.write("x.zr", GOOD);
    let abs = fs::canonicalize(&input).expect("canonicalize");
    assert_refused(&d, Path::new("x.zr"), &abs);
}

#[test]
fn refuses_dot_dot_spelling_of_the_input() {
    let d = Dir::new();
    let input = d.write("x.zr", GOOD);
    fs::create_dir(d.path("sub")).expect("mkdir");
    assert_refused(&d, &input, Path::new("sub/../x.zr"));
}

#[cfg(unix)]
#[test]
fn refuses_symlink_to_the_input() {
    let d = Dir::new();
    let input = d.write("x.zr", GOOD);
    std::os::unix::fs::symlink(&input, d.path("link.html")).expect("symlink");
    assert_refused(&d, &input, &d.path("link.html"));
}

#[cfg(unix)]
#[test]
fn refuses_hard_link_to_the_input() {
    let d = Dir::new();
    let input = d.write("x.zr", GOOD);
    fs::hard_link(&input, d.path("hard.html")).expect("hard link");
    assert_refused(&d, &input, &d.path("hard.html"));
}

#[test]
fn a_different_file_in_the_same_directory_is_fine() {
    let d = Dir::new();
    let input = d.write("x.zr", GOOD);
    d.write("other.html", "old");
    let o = mazer(&[&input, Path::new("-o"), &d.path("other.html")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(fs::read_to_string(d.path("other.html")).expect("read").contains("<mfrac>"));
    assert_eq!(fs::read_to_string(&input).expect("read"), GOOD);
    let o = mazer(&[&input, Path::new("-o"), &d.path("fresh.html")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(d.path("fresh.html").exists());
}

// ---- --check reports what a compile reports ----

const MIXED: &str = "# T\n\n(show (frac 1))\n\n[x](javascript:alert(1))\n\nSee (ref missing).\n\n(eval x)\n";

#[test]
fn check_prints_the_same_diagnostics_as_a_compile() {
    let d = Dir::new();
    let input = d.write("note.zr", MIXED);
    let check = mazer(&[&input, Path::new("--check")]);
    let compile = mazer(&[&input, Path::new("-o"), Path::new("-")]);
    assert_eq!(code(&check), 1);
    assert_eq!(code(&compile), 1);
    assert_eq!(err(&check), err(&compile));
    let e = err(&check);
    for needle in ["error[arity]", "warning[other]", "error[unresolved-ref]", "error[no-evaluator]"] {
        assert!(e.contains(needle), "missing {needle}: {e}");
    }
    assert!(!d.path("note.html").exists());
}

#[test]
fn check_reports_dangerous_urls() {
    let d = Dir::new();
    let input = d.write("note.zr", "[x](javascript:alert(1))\n\n![i](data:text/html,x)\n");
    let o = mazer(&[&input, Path::new("--check")]);
    assert_eq!(code(&o), 0);
    assert_eq!(err(&o).matches("warning[other]").count(), 2, "{}", err(&o));
}

// ---- --strict ----

const WARN_ONLY: &str = "[x](javascript:alert(1))\n";

#[test]
fn strict_makes_warnings_fail() {
    let d = Dir::new();
    let input = d.write("note.zr", WARN_ONLY);
    let o = mazer(&[&input, Path::new("--strict")]);
    assert_eq!(code(&o), 1, "{}", err(&o));
    assert!(err(&o).contains("1 warning (--strict"), "{}", err(&o));
    assert!(d.path("note.html").exists(), "HTML is still written");
    let o = mazer(&[&input, Path::new("--check"), Path::new("--strict")]);
    assert_eq!(code(&o), 1);
    let o = mazer(&[&input, Path::new("--check")]);
    assert_eq!(code(&o), 0);
}

#[test]
fn strict_ignores_hints_and_clean_notes() {
    let d = Dir::new();
    let clean = d.write("clean.zr", GOOD);
    let o = mazer(&[&clean, Path::new("--check"), Path::new("--strict")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(err(&o), "");
    let hint = d.write("hint.zr", "(show (fracc 1 2))\n");
    let o = mazer(&[&hint, Path::new("--check"), Path::new("--strict")]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(err(&o).contains("hint[unknown-form]"), "{}", err(&o));
}

#[test]
fn hint_only_note_prints_a_summary() {
    let d = Dir::new();
    let input = d.write("hint.zr", "(show (fracc 1 2))\n");
    let o = mazer(&[&input, Path::new("--check")]);
    assert_eq!(code(&o), 0);
    assert!(err(&o).trim_end().ends_with("1 hint"), "{}", err(&o));
}

#[test]
fn summary_counts_every_severity() {
    let d = Dir::new();
    let input = d.write("n.zr", "(show (frac 1))\n\n[x](javascript:a)\n\n(show (fracc 1 2))\n");
    let o = mazer(&[&input, Path::new("--check")]);
    assert!(err(&o).trim_end().ends_with("1 error, 1 warning, 1 hint"), "{}", err(&o));
}

// ---- flags and stdin ----

#[test]
fn flags_with_values_are_usage_errors() {
    for args in [
        &["a.zr", "--check=no"][..],
        &["a.zr", "--fragment=0"],
        &["a.zr", "--strict=false"],
        &["a.zr", "--output="],
        &["a.zr", "-o", ""],
    ] {
        let o = mazer_str(args);
        assert_eq!(code(&o), 2, "{args:?}");
        assert!(err(&o).contains("--help"), "{args:?}: {}", err(&o));
    }
    let o = mazer_str(&["a.zr", "--output="]);
    assert!(err(&o).contains("non-empty"), "{}", err(&o));
    let o = mazer_str(&["a.zr", "--check=no"]);
    assert!(err(&o).contains("does not take a value"), "{}", err(&o));
}

#[test]
fn stdin_input_goes_to_stdout() {
    let o = mazer_stdin(&["-"], GOOD);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).starts_with("<!DOCTYPE html>"), "{}", out(&o));
    assert!(out(&o).contains("<mfrac>"));
    let o = mazer_stdin(&["-", "--fragment"], GOOD);
    assert!(!out(&o).contains("<html"), "{}", out(&o));
}

#[test]
fn stdin_diagnostics_use_the_stdin_name() {
    let o = mazer_stdin(&["-", "--check"], BAD);
    assert_eq!(code(&o), 1);
    assert!(err(&o).contains("<stdin>:3:"), "{}", err(&o));
    assert_eq!(out(&o), "");
}

#[test]
fn stdin_with_open_is_a_usage_error() {
    let o = mazer_stdin(&["-", "--open"], GOOD);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("standard input"), "{}", err(&o));
}

#[test]
fn stdin_can_write_to_a_file() {
    let d = Dir::new();
    let target = d.path("from-stdin.html");
    let o = Command::new(env!("CARGO_BIN_EXE_mazer"))
        .arg("-")
        .arg("-o")
        .arg(&target)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run");
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(target.exists());
}

#[test]
fn query_double_dash_ends_options() {
    let o = mazer_str(&["query", "--", "--file"]);
    // Not an unknown-option usage error: it is searched for as text.
    assert_ne!(code(&o), 2, "{}", err(&o));
    assert!(err(&o).contains("nothing matches `--file`"), "{}", err(&o));
}
