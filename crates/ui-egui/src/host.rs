//! The surface an embedding host drives: one window per drawing, document bytes in and bytes
//! out, the host's own wgpu device shared through [`CadApp::set_wgpu`]. The desktop app is one
//! such host (its shell is `apps/cadcraft`); an app that keeps a `CadApp` inside its own window
//! frame — bace's live windows — uses this instead of the eframe setup in `main`.

use serde_json::{Value, json};

use cadcraft_engine::{Session, sample};

use crate::{CadApp, Services};

/// The file name a hosted drawing is saved under: only the extension matters (it picks the
/// writer). The bytes go back to the host's document store; `path` never touches a file system
/// the host may not have.
const HOST_PATH: &str = "document.dwg";

/// The file-format hooks [`Session::execute`] needs for OPEN/SAVE: the wiring the desktop and
/// web apps both installed in their `main`, moved here so every host gets the same setup. The
/// engine installs hooks once; extra calls are ignored.
pub fn install_io() {
    cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
    });
    cadcraft_engine::cmd::file::set_image_decoder(|b| cadcraft_io::raster::decode(b).map_err(|e| e.to_string()));
    cadcraft_engine::cmd::file::set_write_as(|d, name, o| {
        cadcraft_io::write_with(d, name, &cadcraft_io::SaveOptions { version: o.version, binary: o.binary }).map_err(|e| e.to_string())
    });
    // AutoLISP's entget, entmake and entmod see entities as DXF groups.
    cadcraft_engine::cmd::file::set_entity_codec(cadcraft_engine::cmd::file::EntityCodec {
        groups: cadcraft_io::entity_groups,
        entity: cadcraft_io::entity_from_groups,
    });
}

/// An app for a window with no document of its own: the sample drawing the binary's `--sample`
/// opens, so a fresh window has something on the canvas.
pub fn starter() -> CadApp {
    install_io();
    let mut session = Session::empty();
    session.open_drawing(sample::default_sample(), "Drawing1", None);
    finish(CadApp::new(session, Services::default()), true)
}

/// An app opened on `bytes` read as `name`: the document the host's store holds. The host
/// decides what a failed open shows (a window keeps the starter drawing, say); the error reaches
/// it rather than a status bar it may not read.
pub fn app(name: &str, bytes: &[u8]) -> Result<CadApp, String> {
    install_io();
    let mut session = Session::empty();
    let r = session.execute("open", &json!({ "data": cadcraft_engine::cmd::file::base64_encode(bytes), "name": name })).map_err(|e| e.to_string())?;
    // A drawing that opened on the view it was saved at keeps it; the rest fit on the first
    // canvas pass, as `opened` does.
    let fit = !r.get("saved_view").and_then(Value::as_bool).unwrap_or(false);
    Ok(finish(CadApp::new(session, Services::default()), fit))
}

/// The state the app's own `opened` leaves a window in after a successful OPEN: the drawing in
/// front instead of the Start tab, and the canvas told whether the first pass fits the extents.
/// The hosted path skips `opened`'s recent-file bookkeeping and its Block Editor prompt: the
/// host owns where documents come from.
fn finish(mut app: CadApp, fit: bool) -> CadApp {
    app.ui.start_tab = false;
    app.canvas.zoom_pending = fit;
    app
}

impl CadApp {
    /// `(document uid, revision)` of the front drawing — enough for a host to tell an edit or a
    /// document switch from a repaint, and to notice `None` (the Start tab, or every document
    /// closed).
    pub fn document_revision(&self) -> Option<(u64, u64)> {
        self.session.state().ok().map(|st| (st.uid, st.revision))
    }

    /// The front drawing as DWG bytes, marked saved the way QSAVE leaves it: the host keeps the
    /// file. Not a PLOT-style export — the document store holds the drawing itself.
    pub fn save_dwg_bytes(&mut self) -> Result<Vec<u8>, String> {
        install_io();
        cadcraft_engine::cmd::file::save_as_bytes(&mut self.session, HOST_PATH, &cadcraft_engine::cmd::file::SaveAs::default())
            .map_err(|e| e.to_string())
    }
}
