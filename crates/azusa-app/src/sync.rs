use std::sync::{Arc, mpsc};

use azusa_core::anilist::{self, MediaListGroup, Session};
use eframe::egui;

use crate::ui::anime_table::ProgressChange;

/// Sends progress changes to `AniList` in order from a background thread.
pub struct ProgressSync {
	tx: mpsc::Sender<ProgressChange>,
	error_rx: mpsc::Receiver<SyncFailure>,
}

/// A failed progress update. Also contains the lists reloaded after it if the reload worked.
pub struct SyncFailure {
	pub error: anilist::Error,
	pub lists: Option<Vec<MediaListGroup>>,
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
					let lists = fetch_lists(&session)
						.inspect_err(|err| log::warn!("reloading lists failed: {err}"))
						.ok();
					let _ = error_tx.send(SyncFailure { error: err, lists });
					ctx.request_repaint();
				}
			}
		});
		Self { tx, error_rx }
	}

	pub fn send(&self, change: ProgressChange) {
		let _ = self.tx.send(change);
	}

	pub fn poll_error(&self) -> Option<SyncFailure> {
		self.error_rx.try_recv().ok()
	}
}

/// Fetches the anime lists the app supports.
pub fn fetch_lists(session: &Session) -> Result<Vec<MediaListGroup>, anilist::Error> {
	let mut lists = session.anime_lists()?;
	lists.retain(|group| group.is_custom_list != Some(true));
	Ok(lists)
}
