use std::sync::{Arc, mpsc};

use azusa_core::anilist::{self, MediaListGroup, Session};
use eframe::egui;

/// Work queued for the sync thread.
enum Command {
	Progress(ProgressChange),
	Reload,
}

/// A result sent back from the sync thread.
pub enum SyncEvent {
	ProgressFailed(SyncFailure),
	Reloaded(Result<Vec<MediaListGroup>, anilist::Error>),
}

/// A new progress value for a list entry, to be saved on `AniList`.
#[derive(Debug)]
pub struct ProgressChange {
	pub entry_id: i32,
	/// The total number of episodes watched
	pub progress: u32,
}

/// A failed progress update. Also contains the lists reloaded after it if the reload worked.
pub struct SyncFailure {
	pub error: anilist::Error,
	pub lists: Option<Vec<MediaListGroup>>,
}

/// Saves progress changes and reloads the lists in order from a background thread.
pub struct ListSync {
	tx: mpsc::Sender<Command>,
	event_rx: mpsc::Receiver<SyncEvent>,
}

impl ListSync {
	pub fn spawn(ctx: &egui::Context, session: Arc<Session>) -> Self {
		let (tx, commands) = mpsc::channel::<Command>();
		let (event_tx, event_rx) = mpsc::channel();
		let ctx = ctx.clone();
		std::thread::spawn(move || {
			for command in &commands {
				match command {
					Command::Reload => {
						let _ = event_tx.send(SyncEvent::Reloaded(fetch_lists(&session)));
						ctx.request_repaint();
					}
					Command::Progress(change) => {
						let Ok(progress) = i32::try_from(change.progress) else {
							continue;
						};
						if let Err(err) = session.update_progress(change.entry_id, progress) {
							// Drop pending changes so AniList history has no gaps.
							commands.try_iter().for_each(drop);
							let lists = fetch_lists(&session)
								.inspect_err(|err| log::warn!("reloading lists failed: {err}"))
								.ok();
							let _ = event_tx
								.send(SyncEvent::ProgressFailed(SyncFailure { error: err, lists }));
							ctx.request_repaint();
						}
					}
				}
			}
		});
		Self { tx, event_rx }
	}

	pub fn send(&self, change: ProgressChange) {
		let _ = self.tx.send(Command::Progress(change));
	}

	pub fn reload(&self) {
		let _ = self.tx.send(Command::Reload);
	}

	pub fn poll(&self) -> Option<SyncEvent> {
		self.event_rx.try_recv().ok()
	}
}

/// Fetches the anime lists the app supports.
pub fn fetch_lists(session: &Session) -> Result<Vec<MediaListGroup>, anilist::Error> {
	let mut lists = session.anime_lists()?;
	lists.retain(|group| group.is_custom_list != Some(true));
	Ok(lists)
}
