//! The page's app: PdfCraft as it is, plus autosave to the host in host mode ([`crate::autosave`]).

use pdfcraft_ui_egui::{PdfCraftApp, SaveTarget};
use web_time::Instant;

use crate::autosave::Autosave;

pub struct WebShell {
    app: PdfCraftApp,
    /// Saves to the host once the document rests, and the saves still out; only in host mode.
    autosave: Option<(Autosave, crate::in_flight::Shared)>,
}

impl WebShell {
    /// `saves`: the host bridge's saves out, when a host page takes the saves (`?host=parent` or
    /// `data-host="page"`).
    pub fn new(app: PdfCraftApp, saves: Option<crate::in_flight::Shared>) -> Self {
        Self { app, autosave: saves.map(|saves| (Autosave::new(Instant::now()), saves)) }
    }

    /// Host mode: Save (to the host, as Ctrl+S does) once the document in front has had unsaved
    /// changes and no work for [`crate::autosave::IDLE`]. Only when that Save asks nothing
    /// ([`PdfCraftApp::saves_active_silently`]): autosave never opens a dialog.
    fn autosave(&mut self, ctx: &egui::Context) {
        let Some((autosave, saves)) = &mut self.autosave else { return };
        let now = Instant::now();
        let app = &self.app;
        let front = app.active.and_then(|i| app.views.get(i)).and_then(|v| Some((v.id, app.session.get(v.id)?)));
        let dirty = front.is_some_and(|(_, doc)| doc.dirty) && app.saves_active_silently();
        let busy = crate::autosave::user_busy(ctx);
        // Each edit changes the undo depth (until the history is full; then the edit's own input
        // starts the idle time).
        let front = front.map(|(id, doc)| (id.0, u64::try_from(doc.undo_depth()).unwrap_or(u64::MAX)));
        let saving = saves.borrow_mut().running(now);
        if autosave.step(front, dirty, busy, saving, now) {
            self.app.save_active(SaveTarget::InPlace);
        }
        if let Some(wait) = autosave.wake_in(now) {
            ctx.request_repaint_after(wait);
        }
    }
}

impl eframe::App for WebShell {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        eframe::App::raw_input_hook(&mut self.app, ctx, raw);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::App::save(&mut self.app, storage);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        eframe::App::on_exit(&mut self.app, gl);
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        eframe::App::logic(&mut self.app, ctx, frame);
        self.autosave(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        eframe::App::ui(&mut self.app, ui, frame);
        // Again after the UI: an edit made in it starts its idle time now, not at the next pass,
        // which an idle page may never run.
        self.autosave(ui.ctx());
    }
}
