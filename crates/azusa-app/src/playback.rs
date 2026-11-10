use std::{
	sync::mpsc,
	thread,
	time::{Duration, Instant},
};

use azusa_core::library::{self, MatchedFile};
use eframe::egui;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const FALLBACK_EPISODE_DURATION_MINUTES: u64 = 24;

#[derive(Debug)]
pub struct NowPlaying {
	pub file: MatchedFile,
	pub open: bool,
	pub opened_at: Instant,
}

impl NowPlaying {
	pub fn is_next_episode(&self, progress: Option<i32>) -> bool {
		let progress = progress
			.and_then(|progress| u32::try_from(progress).ok())
			.unwrap_or(0);

		self.file.episode == progress + 1
	}
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

pub fn watched_enough(
	time_open: Duration,
	episode_duration_minutes: Option<i32>,
	required_percent: u8,
) -> bool {
	let duration_seconds = episode_duration_minutes
		.and_then(|minutes| u64::try_from(minutes).ok())
		.unwrap_or(FALLBACK_EPISODE_DURATION_MINUTES)
		* 60;

	(time_open.as_secs() * 100) >= (duration_seconds * u64::from(required_percent))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn counts_as_watched_after_percent_of_duration() {
		let cases = [
			(Duration::from_mins(2), Some(24), 80, false),
			(Duration::from_mins(23), Some(24), 80, true),
			(Duration::from_mins(54), Some(24), 80, true),
			(Duration::from_mins(6), Some(12), 80, false),
			(Duration::from_mins(10), Some(12), 80, true),
			(Duration::from_mins(35), Some(100), 80, false),
			(Duration::from_mins(105), Some(100), 80, true),
			(Duration::from_mins(2), None, 80, false),
			(Duration::from_mins(24), None, 80, true),
			(Duration::from_mins(24), Some(-4), 80, true),
		];

		for (time_open, episode_duration_minutes, required_percent, expected) in cases {
			assert_eq!(
				watched_enough(time_open, episode_duration_minutes, required_percent),
				expected,
				"{time_open:?} of {episode_duration_minutes:?} min at {required_percent}%"
			);
		}
	}

	#[test]
	fn only_the_next_episode_counts() {
		let cases = [
			(Some(4), 5, true),
			(None, 1, true),
			(Some(4), 7, false),
			(Some(4), 3, false),
		];

		for (progress, episode, expected) in cases {
			let playing = NowPlaying {
				file: MatchedFile {
					media_id: 1,
					episode,
				},
				open: true,
				opened_at: Instant::now(),
			};

			assert_eq!(
				playing.is_next_episode(progress),
				expected,
				"episode {episode} after {progress:?}"
			);
		}
	}
}
