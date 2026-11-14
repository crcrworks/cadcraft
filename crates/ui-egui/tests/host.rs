//! The hosted surface an embedding app drives (`host::app`/`starter`, document bytes in and
//! out). Its own test binary so this file's `install_io` is the only io setup it sees — the
//! engine's hooks are a `OnceLock`.

use cadcraft_ui_egui::CadApp;
use cadcraft_ui_egui::host;
use serde_json::json;

/// A line through the command JSON every caller can send.
fn draw_line(app: &mut CadApp) {
    app.session.execute("line", &json!({ "points": [[0.0, 0.0], [10.0, 0.0]] })).unwrap();
}

fn entities(app: &CadApp) -> usize {
    app.session.state().unwrap().doc.entity_count()
}

#[test]
fn the_starter_has_a_document() {
    let app = host::starter();
    assert!(app.document_revision().is_some());
    assert!(!app.ui.start_tab);
    assert!(entities(&app) > 0);
}

#[test]
fn a_document_opens_saves_and_reopens_as_dwg() {
    let mut app = host::starter();
    let before = entities(&app);
    draw_line(&mut app);
    let bytes = app.save_dwg_bytes().unwrap();
    assert!(!bytes.is_empty());
    // Saved means clean: the revision the host stored is the one the window stays on.
    assert!(!app.session.state().unwrap().is_dirty());
    let reopened = host::app("document.dwg", &bytes).unwrap();
    assert_eq!(entities(&reopened), before + 1);
    assert!(!reopened.session.state().unwrap().is_dirty());
}

#[test]
fn an_unreadable_document_is_an_error_not_a_panic() {
    assert!(host::app("document.dwg", b"not a drawing").is_err());
}
