//! `mazer`: compile `.zr` notes to HTML.

mod args;
mod query;
mod report;

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode, Stdio};

use mazer_doc::{Diag, DiagCode, Diags, Options, Page, Severity, SourceMap, Target};

use args::{Command, CompileArgs, TargetArg, USAGE};

/// Exit code for usage and I/O errors.
const EXIT_FATAL: u8 = 2;

fn main() -> ExitCode {
    match args::parse(std::env::args().skip(1)) {
        Ok(Command::Help) => stdout_or_exit(USAGE),
        Ok(Command::Version) => stdout_or_exit(&format!("mazer {}\n", env!("CARGO_PKG_VERSION"))),
        Ok(Command::Compile(c)) => compile_command(&c),
        Ok(Command::Query { text, file }) => query_command(&text, file.as_deref()),
        Err(e) => {
            eprintln!("mazer: {e}");
            eprintln!("Try `mazer --help` for usage.");
            ExitCode::from(EXIT_FATAL)
        }
    }
}

fn stdout_or_exit(text: &str) -> ExitCode {
    match write_stdout(text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => io_failure("cannot write to stdout", &e),
    }
}

/// A closed pipe (`mazer query x | head`) is not an error.
fn write_stdout(text: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

fn io_failure(what: &str, e: &io::Error) -> ExitCode {
    eprintln!("mazer: {what}: {e}");
    ExitCode::from(EXIT_FATAL)
}

fn color_wanted() -> bool {
    io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
}

fn read_note(path: &Path) -> Result<String, ExitCode> {
    std::fs::read_to_string(path)
        .map_err(|e| io_failure(&format!("cannot read `{}`", path.display()), &e))
}

const fn target_of(t: TargetArg) -> Target {
    match t {
        TargetArg::MathMl => Target::MathMl,
        #[cfg(feature = "latex")]
        TargetArg::Latex => Target::Latex,
    }
}

/// Where the HTML goes: `Some(path)`, or `None` for stdout.
fn output_path(c: &CompileArgs) -> Option<PathBuf> {
    match &c.output {
        Some(p) if p.as_os_str() == "-" => None,
        Some(p) => Some(p.clone()),
        None => Some(c.input.with_extension("html")),
    }
}

fn print_diags(diags: &[Diag], sources: &SourceMap) {
    let color = color_wanted();
    let mut err = io::stderr().lock();
    // In reading order, the note's own file before the embedded prelude.
    let mut sorted: Vec<&Diag> = diags.iter().collect();
    sorted.sort_by_key(|d| (d.span.file.0, d.span.start));
    for d in sorted {
        // A closed stderr leaves nowhere to complain, so ignore write errors.
        let _ = writeln!(err, "{}\n", report::render_diag(d, sources, color));
    }
    if let Some(line) = report::summary(diags) {
        let _ = writeln!(err, "{line}");
    }
}

fn compile_command(c: &CompileArgs) -> ExitCode {
    let src = match read_note(&c.input) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let name = c.input.display().to_string();
    let stem = c
        .input
        .file_stem()
        .map_or_else(|| name.clone(), |s| s.to_string_lossy().into_owned());
    let mut opts = Options::new(name.clone());
    opts.target = target_of(c.target);
    opts.page = if c.fragment {
        Page::Fragment
    } else {
        Page::Full { title: stem }
    };

    if c.check {
        let (diags, sources) = check(&src, &name, &mut opts);
        print_diags(&diags, &sources);
        return exit_for(&diags);
    }

    let dest = output_path(c);
    if dest.as_deref() == Some(c.input.as_path()) {
        eprintln!(
            "mazer: refusing to overwrite the input `{}`; choose another output with -o",
            c.input.display()
        );
        return ExitCode::from(EXIT_FATAL);
    }

    let out = mazer_doc::compile(&src, &mut opts);
    print_diags(&out.diags, &out.sources);
    let code = exit_for(&out.diags);

    let written = match &dest {
        None => write_stdout(&out.html).map_err(|e| ("cannot write to stdout".to_owned(), e)),
        Some(path) => std::fs::write(path, out.html.as_bytes())
            .map_err(|e| (format!("cannot write `{}`", path.display()), e)),
    };
    if let Err((what, e)) = written {
        return io_failure(&what, &e);
    }
    if c.open
        && let Some(path) = &dest
        && let Err(e) = open_in_browser(path)
    {
        eprintln!("mazer: could not open `{}`: {e}", path.display());
    }
    code
}

/// `analyze` plus the one problem only `render` can find: `(eval ...)` with no
/// evaluator. Nothing else `render` reports is kept.
fn check(src: &str, name: &str, opts: &mut Options) -> (Vec<Diag>, SourceMap) {
    let analysis = mazer_doc::analyze(src, name);
    let mut diags = analysis.diags.clone().into_vec();
    let mut rendered = Diags::new();
    let _ = mazer_doc::render(&analysis, opts, &mut rendered);
    diags.extend(
        rendered
            .into_vec()
            .into_iter()
            .filter(|d| d.code == DiagCode::NoEvaluator),
    );
    (diags, analysis.sources)
}

fn exit_for(diags: &[Diag]) -> ExitCode {
    if diags.iter().any(|d| d.severity == Severity::Error) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn open_in_browser(path: &Path) -> io::Result<()> {
    let program = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    Process::new(program)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

fn query_command(text: &str, file: Option<&Path>) -> ExitCode {
    let analysis = match file {
        None => None,
        Some(path) => match read_note(path) {
            Ok(src) => Some(mazer_doc::analyze(&src, &path.display().to_string())),
            Err(code) => return code,
        },
    };
    let empty = SourceMap::new();
    let hits = analysis.as_ref().map_or_else(
        || query::search(text, [], &empty),
        |a| query::search(text, a.macros.iter(), &a.sources),
    );
    if hits.is_empty() {
        eprintln!("mazer: nothing matches `{text}`");
        return ExitCode::from(1);
    }
    let body = hits
        .iter()
        .map(|h| h.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    stdout_or_exit(&body)
}
