//! Hand-written argument parsing. Pure: no I/O, so every case is unit-tested.

use std::path::PathBuf;

pub const USAGE: &str = "\
Usage:
    mazer <file.zr> [options]
    mazer query <text...> [--file <note.zr>]
    mazer --help | -h
    mazer --version | -V

Compile a .zr note (markdown with embedded Lisp math) to HTML.

Options:
    -o, --output <path>   Where to write (default: next to the input, same name
                          with .html; `-o -` writes to stdout)
        --open            Open the result in the default browser
        --fragment        Inner HTML only, no page wrapper (default is a full
                          page whose title is the file stem)
        --target <t>      `mathml` (default) or `latex`; `latex` needs a build
                          with `--features latex`
        --check           Report diagnostics only and write nothing. Exit code 1
                          if there are errors. Also reports `(eval ...)` forms,
                          since no evaluator is built in
    -h, --help            Show this help
    -V, --version         Show the version

Query:
    mazer query <text...>   Search keywords, display names and docs
                            (case-insensitive). `-q` / `--query` also work.
        --file <note.zr>    Also search that note's macros and aliases

Diagnostics go to stderr. Exit codes: 0 success (warnings do not fail),
1 the note has errors (the HTML is still written), 2 usage or I/O error.
";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetArg {
    MathMl,
    #[cfg(feature = "latex")]
    Latex,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompileArgs {
    pub input: PathBuf,
    /// `Some("-")` is stdout.
    pub output: Option<PathBuf>,
    pub open: bool,
    pub fragment: bool,
    pub target: TargetArg,
    pub check: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Command {
    Help,
    Version,
    Compile(CompileArgs),
    Query { text: String, file: Option<PathBuf> },
}

/// A usage error: printed with a pointer to `--help`, exit code 2.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UsageError(pub String);

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(msg: impl Into<String>) -> Result<T, UsageError> {
    Err(UsageError(msg.into()))
}

/// Parses everything after the program name.
///
/// # Errors
/// Returns a [`UsageError`] describing the first problem found.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, UsageError> {
    let args: Vec<String> = args.into_iter().collect();
    match args.first().map(String::as_str) {
        None => err("no input file given"),
        Some("query" | "-q" | "--query") => parse_query(&args[1..]),
        Some(_) => parse_compile(&args),
    }
}

fn parse_query(args: &[String]) -> Result<Command, UsageError> {
    let mut words: Vec<&str> = Vec::new();
    let mut file = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--file" => match it.next() {
                Some(v) => file = Some(PathBuf::from(v)),
                None => return err("`--file` needs a path"),
            },
            a if a.starts_with("--file=") => file = Some(PathBuf::from(&a["--file=".len()..])),
            "-h" | "--help" => return Ok(Command::Help),
            a if a.starts_with('-') && a.len() > 1 => {
                return err(format!("unknown option `{a}` for `query`"));
            }
            a => words.push(a),
        }
    }
    let text = words.join(" ");
    if text.trim().is_empty() {
        return err("`query` needs some text to search for");
    }
    Ok(Command::Query { text, file })
}

fn parse_target(value: &str) -> Result<TargetArg, UsageError> {
    match value {
        "mathml" => Ok(TargetArg::MathMl),
        #[cfg(feature = "latex")]
        "latex" => Ok(TargetArg::Latex),
        #[cfg(not(feature = "latex"))]
        "latex" => err(
            "this build of mazer has no LaTeX support; rebuild with `--features latex` \
             (for example `cargo install --force --path ./mazer-cli --features latex`)",
        ),
        other => err(format!(
            "unknown target `{other}` (expected `mathml` or `latex`)"
        )),
    }
}

fn parse_compile(args: &[String]) -> Result<Command, UsageError> {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut open = false;
    let mut fragment = false;
    let mut check = false;
    let mut target = TargetArg::MathMl;
    let mut only_positional = false;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let arg = arg.as_str();
        // `--name=value` is the same as `--name value`.
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_owned())),
            _ => (arg, None),
        };
        let mut value = |name: &str| -> Result<String, UsageError> {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .map_or_else(|| err(format!("`{name}` needs a value")), Ok)
        };
        if only_positional || !arg.starts_with('-') || arg == "-" {
            if input.is_some() {
                return err(format!("unexpected extra argument `{arg}`"));
            }
            input = Some(PathBuf::from(arg));
            continue;
        }
        match flag {
            "--" => only_positional = true,
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-o" | "--output" => output = Some(PathBuf::from(value(flag)?)),
            "--target" => target = parse_target(&value(flag)?)?,
            "--open" => open = true,
            "--fragment" => fragment = true,
            "--check" => check = true,
            _ => return err(format!("unknown option `{arg}`")),
        }
    }

    let Some(input) = input else {
        return err("no input file given");
    };
    if check && output.is_some() {
        return err("`--check` writes nothing, so it cannot be combined with `--output`");
    }
    if check && open {
        return err("`--check` writes nothing, so it cannot be combined with `--open`");
    }
    if open && output.as_deref().is_some_and(|p| p.as_os_str() == "-") {
        return err("`--open` needs a file, so it cannot be combined with `-o -`");
    }
    Ok(Command::Compile(CompileArgs {
        input,
        output,
        open,
        fragment,
        target,
        check,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(|s| (*s).to_owned()))
    }

    fn compile(args: &[&str]) -> CompileArgs {
        match p(args) {
            Ok(Command::Compile(c)) => c,
            other => panic!("expected compile, got {other:?}"),
        }
    }

    fn usage_err(args: &[&str]) -> String {
        match p(args) {
            Err(e) => e.0,
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn defaults() {
        let c = compile(&["a.zr"]);
        assert_eq!(c.input, PathBuf::from("a.zr"));
        assert_eq!(c.output, None);
        assert!(!c.open && !c.fragment && !c.check);
        assert_eq!(c.target, TargetArg::MathMl);
    }

    #[test]
    fn output_flags() {
        assert_eq!(compile(&["a.zr", "-o", "x.html"]).output, Some("x.html".into()));
        assert_eq!(compile(&["a.zr", "--output", "x.html"]).output, Some("x.html".into()));
        assert_eq!(compile(&["a.zr", "--output=x.html"]).output, Some("x.html".into()));
        assert_eq!(compile(&["-o", "-", "a.zr"]).output, Some("-".into()));
        assert!(usage_err(&["a.zr", "-o"]).contains("needs a value"));
    }

    #[test]
    fn boolean_flags_and_order() {
        let c = compile(&["--fragment", "a.zr", "--open"]);
        assert!(c.fragment && c.open && !c.check);
        let c = compile(&["--check", "a.zr"]);
        assert!(c.check);
    }

    #[test]
    fn target() {
        assert_eq!(compile(&["a.zr", "--target", "mathml"]).target, TargetArg::MathMl);
        assert!(usage_err(&["a.zr", "--target", "docx"]).contains("unknown target"));
        assert!(usage_err(&["a.zr", "--target"]).contains("needs a value"));
        #[cfg(feature = "latex")]
        assert_eq!(compile(&["a.zr", "--target=latex"]).target, TargetArg::Latex);
        #[cfg(not(feature = "latex"))]
        assert!(usage_err(&["a.zr", "--target", "latex"]).contains("--features latex"));
    }

    #[test]
    fn help_and_version() {
        assert_eq!(p(&["--help"]), Ok(Command::Help));
        assert_eq!(p(&["-h"]), Ok(Command::Help));
        assert_eq!(p(&["a.zr", "-h"]), Ok(Command::Help));
        assert_eq!(p(&["--version"]), Ok(Command::Version));
        assert_eq!(p(&["-V"]), Ok(Command::Version));
    }

    #[test]
    fn errors() {
        assert!(usage_err(&[]).contains("no input"));
        assert!(usage_err(&["--fragment"]).contains("no input"));
        assert!(usage_err(&["a.zr", "b.zr"]).contains("extra argument"));
        assert!(usage_err(&["a.zr", "--wat"]).contains("unknown option"));
        assert!(usage_err(&["a.zr", "-x"]).contains("unknown option"));
        assert!(usage_err(&["a.zr", "--check", "-o", "x"]).contains("--check"));
        assert!(usage_err(&["a.zr", "--check", "--open"]).contains("--check"));
        assert!(usage_err(&["a.zr", "--open", "-o", "-"]).contains("--open"));
    }

    #[test]
    fn double_dash_allows_odd_names() {
        assert_eq!(compile(&["--", "-weird.zr"]).input, PathBuf::from("-weird.zr"));
    }

    #[test]
    fn query_forms() {
        let q = |args: &[&str]| match p(args) {
            Ok(Command::Query { text, file }) => (text, file),
            other => panic!("expected query, got {other:?}"),
        };
        assert_eq!(q(&["query", "integral"]), ("integral".into(), None));
        assert_eq!(q(&["query", "real", "numbers"]).0, "real numbers");
        assert_eq!(q(&["-q", "tau"]).0, "tau");
        assert_eq!(q(&["--query", "tau"]).0, "tau");
        assert_eq!(
            q(&["query", "x", "--file", "n.zr"]),
            ("x".into(), Some("n.zr".into()))
        );
        assert_eq!(q(&["query", "--file=n.zr", "x"]).1, Some("n.zr".into()));
        assert!(usage_err(&["query"]).contains("needs some text"));
        assert!(usage_err(&["query", "--file"]).contains("needs a path"));
        assert!(usage_err(&["query", "x", "--bad"]).contains("unknown option"));
        assert_eq!(p(&["query", "--help"]), Ok(Command::Help));
    }
}
