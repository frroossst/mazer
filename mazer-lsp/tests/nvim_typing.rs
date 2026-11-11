//! Runs `nvim/test_typing.lua` in a headless Neovim against the built
//! server: it types into `.zr` buffers key by key with the completion menu
//! popping up, and checks the buffer holds exactly what was typed (and that
//! choosing an item on purpose inserts it).
//!
//! Skipped when no `nvim` is found (on PATH, or at
//! `/opt/nvim-linux-x86_64/bin/nvim`), or when it is older than 0.11.

use std::path::{Path, PathBuf};
use std::process::Command;

fn nvim() -> Option<PathBuf> {
    let candidates = std::env::var_os("PATH")
        .map_or_else(Vec::new, |path| std::env::split_paths(&path).map(|dir| dir.join("nvim")).collect());
    candidates
        .into_iter()
        .chain([PathBuf::from("/opt/nvim-linux-x86_64/bin/nvim")])
        .find(|p| p.is_file())
        .filter(|p| recent_enough(p))
}

/// Whether `nvim --version` is 0.11 or newer (built-in LSP completion).
fn recent_enough(nvim: &Path) -> bool {
    let Ok(out) = Command::new(nvim).arg("--version").output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let Some(version) = text.lines().next().and_then(|l| l.strip_prefix("NVIM v")) else {
        return false;
    };
    let mut parts = version.split(['.', '-']).map(|p| p.parse::<u32>().unwrap_or(0));
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    (major, minor) >= (0, 11)
}

#[test]
fn typing_in_neovim_inserts_exactly_what_was_typed() {
    let Some(nvim) = nvim() else {
        eprintln!("skipping: no nvim 0.11+ found");
        return;
    };
    let server = PathBuf::from(env!("CARGO_BIN_EXE_mazer-lsp"));
    let bin_dir = server.parent().expect("the binary's directory");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs = vec![bin_dir.to_path_buf()];
    dirs.extend(std::env::split_paths(&path));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("nvim/test_typing.lua");
    let out = Command::new(&nvim)
        .args(["--clean", "--headless", "-c"])
        .arg(format!("luafile {}", script.display()))
        .env("PATH", std::env::join_paths(dirs).expect("a valid PATH"))
        .output()
        .expect("nvim runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    eprintln!("{stdout}{stderr}");
    assert!(out.status.success(), "the typing script failed ({}):\n{stdout}\n{stderr}", out.status);
    assert!(stdout.contains("all cases passed"), "{stdout}");
}
