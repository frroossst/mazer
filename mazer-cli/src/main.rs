//! `mazer`: compile `.zr` notes to HTML.

mod args;
mod query;
mod report;
mod search;

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode, Stdio};

use mazer_doc::{Diag, Options, Page, Severity, SourceMap, Target};

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
    if is_stdin(path) {
        return io::read_to_string(io::stdin())
            .map_err(|e| io_failure("cannot read standard input", &e));
    }
    std::fs::read_to_string(path)
        .map_err(|e| io_failure(&format!("cannot read `{}`", path.display()), &e))
}

/// The input `-` means standard input.
fn is_stdin(path: &Path) -> bool {
    path.as_os_str() == "-"
}

/// Whether writing to `dest` would overwrite `input`: the same file reached by
/// another spelling (`./x`, an absolute path, `a/../x`), a symlink or a hard
/// link. An output that does not exist yet is judged by its directory and name.
fn same_file(input: &Path, dest: &Path) -> bool {
    let (Ok(src_canon), Ok(src_meta)) = (std::fs::canonicalize(input), std::fs::metadata(input))
    else {
        return false;
    };
    if let Ok(dest_meta) = std::fs::metadata(dest) {
        if std::fs::canonicalize(dest).is_ok_and(|c| c == src_canon) {
            return true;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            return dest_meta.dev() == src_meta.dev() && dest_meta.ino() == src_meta.ino();
        }
        #[cfg(not(unix))]
        {
            let _ = (dest_meta, src_meta);
            return false;
        }
    }
    let parent = match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    match (std::fs::canonicalize(parent), dest.file_name()) {
        (Ok(dir), Some(name)) => dir.join(name) == src_canon,
        _ => false,
    }
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
        None if is_stdin(&c.input) => None,
        None => Some(c.input.with_extension("html")),
    }
}

fn print_diags(diags: &[Diag], sources: &SourceMap, strict: bool) {
    let color = color_wanted();
    // In reading order, the note's own file before the embedded prelude.
    let mut sorted: Vec<&Diag> = diags.iter().collect();
    sorted.sort_by_key(|d| (d.span.file.0, d.span.start));
    let mut text = String::new();
    for d in sorted {
        text.push_str(&report::render_diag(d, sources, color));
        text.push_str("\n\n");
    }
    if let Some(line) = report::summary(diags, strict) {
        text.push_str(&line);
        text.push('\n');
    }
    // A closed stderr leaves nowhere to complain, so ignore write errors.
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

fn compile_command(c: &CompileArgs) -> ExitCode {
    let stdin = is_stdin(&c.input);
    let name = if stdin { "<stdin>".to_owned() } else { c.input.display().to_string() };
    let dest = if c.check { None } else { output_path(c) };
    if let Some(path) = &dest
        && !stdin
        && same_file(&c.input, path)
    {
        eprintln!(
            "mazer: refusing to overwrite the input `{}` (the output `{}` is the same file); \
             choose another output with -o",
            c.input.display(),
            path.display()
        );
        return ExitCode::from(EXIT_FATAL);
    }
    let src = match read_note(&c.input) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let stem = if stdin {
        "stdin".to_owned()
    } else {
        c.input
            .file_stem()
            .map_or_else(|| name.clone(), |s| s.to_string_lossy().into_owned())
    };
    let mut opts = Options::new(name);
    opts.target = target_of(c.target);
    opts.page = if c.fragment {
        Page::Fragment
    } else {
        Page::Full { title: stem }
    };

    // `--check` is a full compile whose HTML is thrown away, so it reports
    // exactly what a normal run reports.
    let mut out = mazer_doc::compile(&src, &mut opts);
    print_diags(&out.diags, &out.sources, c.strict);
    let code = exit_for(&out.diags, c.strict);
    if c.check {
        return code;
    }

    // A full page also gets the equation search box (Ctrl+K or `/`).
    if !c.fragment {
        out.html = search::with_search(&out.html);
    }
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

/// 1 when the note has errors, or warnings under `--strict`; hints never fail.
fn exit_for(diags: &[Diag], strict: bool) -> ExitCode {
    let fails = |d: &Diag| match d.severity {
        Severity::Error => true,
        Severity::Warning => strict,
        Severity::Hint => false,
    };
    if diags.iter().any(fails) {
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
