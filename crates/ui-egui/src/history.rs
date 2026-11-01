//! WoodCraft Phase 9: drives [`cadcraft_engine::history::Historian`] every frame and on exit. The
//! engine decides what to commit and does the git work ([`cadcraft_engine::history`]); this just
//! calls it at the right times, the same way [`crate::autosave`] drives autosave. On the web
//! there is no git history (the engine's `Historian` is native-only).

use crate::CadApp;

impl CadApp {
    /// Every frame: commit what is due, and wake up when the next one is.
    pub fn history_tick(&mut self, _ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // `tick` takes `&Session`, so it can't run on `self.session.history` in place (that
            // would borrow `self.session` both ways at once): pull it out, tick, put it back.
            let mut h = std::mem::take(&mut self.session.history);
            let wait = h.tick(&self.session);
            self.session.history = h;
            if let Some(wait) = wait {
                _ctx.request_repaint_after(wait);
            }
        }
    }

    /// The app closes normally: commit every dirty `.wood` project now, instead of waiting for
    /// the timer.
    pub fn shutdown_history(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut h = std::mem::take(&mut self.session.history);
            h.shutdown(&self.session);
            self.session.history = h;
        }
    }
}
