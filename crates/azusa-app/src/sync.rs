use std::sync::{Arc, mpsc};

use azusa_core::anilist::{self, Session};
use eframe::egui;

use crate::ui::anime_table::ProgressChange;

/// Sends progress changes to `AniList` in order from a background thread.
pub struct ProgressSync {
	tx: mpsc::Sender<ProgressChange>,
	error_rx: mpsc::Receiver<anilist::Error>,
}

impl ProgressSync {
	pub fn spawn(ctx: &egui::Context, session: Arc<Session>) -> Self {
		let (tx, commands) = mpsc::channel::<ProgressChange>();
		let (error_tx, error_rx) = mpsc::channel();
		let ctx = ctx.clone();
		std::thread::spawn(move || {
			for change in &commands {
				let Ok(progress) = i32::try_from(change.progress) else {
					continue;
				};
				if let Err(err) = session.update_progress(change.entry_id, progress) {
					// Drop pending changes so AniList history has no gaps.
					commands.try_iter().for_each(drop);
					let _ = error_tx.send(err);
					ctx.request_repaint();
				}
			}
		});
		Self { tx, error_rx }
	}

	pub fn send(&self, change: ProgressChange) {
		let _ = self.tx.send(change);
	}

	pub fn poll_error(&self) -> Option<anilist::Error> {
		self.error_rx.try_recv().ok()
	}
}
