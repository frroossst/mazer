//! Smoke test over the real notes: every feature at every character
//! boundary of every note, without a panic, and well-formed semantic tokens.
//!
//! The notes are private and never committed. They are read from the
//! directory in `MAZER_FIXTURES`, else `../fixtures/notes`; the test skips
//! when neither exists.

use std::path::PathBuf;

use lsp_types::Uri;
use mazer_doc::{ByteOffset, Page, analyze};
use mazer_lsp::{completion, convert, diagnostics, hover, navigation, semantic, signature, symbols};

fn fixtures() -> Option<PathBuf> {
    let dir = std::env::var_os("MAZER_FIXTURES")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/notes"), PathBuf::from);
    dir.is_dir().then_some(dir)
}

#[test]
fn every_feature_at_every_offset_of_every_note() {
    let Some(dir) = fixtures() else {
        eprintln!("skipping: no fixture notes (set MAZER_FIXTURES)");
        return;
    };
    let mut notes: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("readable fixtures")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "zr"))
        .collect();
    notes.sort();
    assert!(!notes.is_empty(), "no .zr notes in {}", dir.display());
    let uri: Uri = "file:///fixtures/note.zr".parse().unwrap();
    let mut answered = [0usize; 4];
    for path in &notes {
        let text = std::fs::read_to_string(path).expect("a UTF-8 note");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let analysis = analyze(&text, &name);

        let tokens = semantic::semantic_tokens(&analysis);
        if let Err(problem) = semantic::check_well_formed(&text, &tokens) {
            panic!("{name}: {problem}");
        }
        let _ = symbols::document_symbols(&analysis);
        let (_, extra) = diagnostics::render_problems(&analysis, &name, Page::Fragment);
        let _ = diagnostics::collect(&analysis, &uri, &extra);

        let config = completion::Config { snippets: true };
        for at in (0..=text.len()).filter(|&i| text.is_char_boundary(i)) {
            let offset = ByteOffset::from_usize(at);
            // The position round trip is exact at every character boundary
            // (a `\r` before a `\n` aside).
            let pos = convert::position(&analysis.sources, convert::NOTE, offset);
            let back = convert::offset_at(&analysis.sources, pos).to_usize();
            assert!(back == at || text.as_bytes().get(back) == Some(&b'\r'), "{name}: {at} -> {pos:?} -> {back}");

            answered[0] += usize::from(!completion::complete(&analysis, offset, config).is_empty());
            answered[1] += usize::from(hover::hover(&analysis, offset).is_some());
            answered[2] += usize::from(signature::signature_help(&analysis, offset).is_some());
            answered[3] += usize::from(navigation::definition(&analysis, &uri, offset).is_some());
        }
    }
    eprintln!(
        "{} notes; offsets with completion {}, hover {}, signature help {}, definition {}",
        notes.len(),
        answered[0],
        answered[1],
        answered[2],
        answered[3]
    );
    assert!(answered[0] > 0 && answered[1] > 0 && answered[2] > 0);
}
