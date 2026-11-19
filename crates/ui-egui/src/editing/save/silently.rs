//! Whether Save of the tab in front would write it now with nothing to ask: what a web shell
//! checks before it saves on its own (autosave in host mode, `apps/pdfcraft-web/src/autosave.rs`).

use super::{PdfCraftApp, restores_saved_redactions};

impl PdfCraftApp {
    /// Save of the tab in front ([`Self::save_active`] with [`super::SaveTarget::InPlace`]) would
    /// write it now, under its own name, asking nothing: no dialog over the window, no Apply
    /// redactions to deliver as a new file or undone past a save (each makes Save ask where), and
    /// no question whether to lock a Fill & Sign signature into the page.
    pub fn saves_active_silently(&self) -> bool {
        let Some(view) = self.active.and_then(|i| self.views.get(i)) else { return false };
        let Some(doc) = self.session.get(view.id) else { return false };
        let asks_where = view.redactions_applied > view.redactions_downloaded || restores_saved_redactions(view);
        // The same question `save_then` asks.
        let asks_to_lock = self.ask_to_lock_signatures
            && !self.flatten_fill_sign_on_save
            && self.placed_signatures.contains(&view.id)
            && doc.info.annotations.iter().any(|a| a.fill_sign)
            && !doc.is_signed()
            && self.lock_answered != Some(view.id);
        !self.modal_open() && !asks_where && !asks_to_lock
    }
}

#[cfg(test)]
mod tests {
    use crate::PdfCraftApp;

    fn opened() -> PdfCraftApp {
        let pdf = pdfcraft_engine::Session::new().create_from_text("t", "One page").unwrap().to_vec();
        let mut app = PdfCraftApp::new();
        app.open_bytes("doc.pdf", None, pdf).unwrap();
        app
    }

    #[test]
    fn nothing_in_front_is_nothing_to_save() {
        assert!(!PdfCraftApp::new().saves_active_silently());
    }

    #[test]
    fn an_opened_document_saves_silently() {
        assert!(opened().saves_active_silently());
    }

    #[test]
    fn redactions_not_yet_delivered_ask_where() {
        let mut app = opened();
        let i = app.active.unwrap();
        app.views[i].redactions_applied = 1;
        assert!(!app.saves_active_silently());
        app.views[i].redactions_downloaded = 1;
        assert!(app.saves_active_silently(), "delivered once, Save keeps the name");
    }

    #[test]
    fn an_open_dialog_waits() {
        let mut app = opened();
        app.dialog = Some(crate::Dialog::LockSignatures);
        assert!(!app.saves_active_silently());
    }
}
