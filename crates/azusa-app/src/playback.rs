use std::{sync::mpsc, thread, time::Duration};

use azusa_core::library::{self, MatchedFile};
use eframe::egui;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct NowPlaying {
	pub file: MatchedFile,
	pub open: bool,
}

/// Monitors the video open in a player and sends its file name whenever it changes.
pub fn spawn_monitor(ctx: &egui::Context) -> mpsc::Receiver<Option<String>> {
	let (tx, rx) = mpsc::channel();
	let ctx = ctx.clone();
	thread::spawn(move || {
		let mut current = None;
		loop {
			let playing = library::now_playing();
			if playing != current {
				current.clone_from(&playing);
				if tx.send(playing).is_err() {
					break;
				}
				ctx.request_repaint();
			}
			thread::sleep(POLL_INTERVAL);
		}
	});
	rx
}
