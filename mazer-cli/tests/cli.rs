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
    assert!(e.contains("1 error, 0 warnings"), "{e}");
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
