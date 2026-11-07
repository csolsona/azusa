use azusa_core::anilist::MediaList;
use eframe::egui;

use crate::{playback::NowPlaying, sync::ProgressChange};

pub fn show(
	ui: &mut egui::Ui,
	playing: Option<&NowPlaying>,
	entry: Option<&MediaList>,
) -> Option<ProgressChange> {
	if let Some(playing) = playing
		&& let Some(entry) = entry
	{
		let title = entry
			.media
			.as_ref()
			.and_then(|media| media.title.as_ref())
			.and_then(|title| title.romaji.as_deref())
			.unwrap_or("Unknown title");

		ui.heading(title);

		let state = if playing.open {
			"Now playing"
		} else {
			"Last played"
		};
		ui.label(format!("{state}: Episode {}", playing.file.episode));

		let progress = u32::try_from(entry.progress.unwrap_or(0)).unwrap_or(0);
		let total = entry
			.media
			.as_ref()
			.and_then(|media| media.episodes)
			.and_then(|total| u32::try_from(total).ok())
			.filter(|&total| total > 0);
		let entry_id = entry.id;
		let mut change = None;

		ui.horizontal(|ui| {
			let minus = ui.add_enabled(progress > 0, egui::Button::new("-"));
			if minus.clicked() {
				change = Some(ProgressChange {
					entry_id,
					progress: progress - 1,
				});
			}

			ui.label(format!(
				"{progress} / {}",
				total.map_or("?".to_owned(), |total| total.to_string())
			));

			let can_increment = total.is_none_or(|total| progress < total);
			let plus = ui.add_enabled(can_increment, egui::Button::new("+"));
			if plus.clicked() {
				change = Some(ProgressChange {
					entry_id,
					progress: progress + 1,
				});
			}
		});

		change
	} else {
		ui.label("Nothing playing");
		None
	}
}
