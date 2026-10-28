use std::collections::HashMap;

use azusa_core::{
	anilist::{MediaFormat, MediaList, MediaListGroup, MediaSeason},
	library::LocalEpisode,
};
use eframe::egui::{self, Layout};
use egui_extras::{Column, TableBuilder, TableRow};

pub fn show(
	ui: &mut egui::Ui,
	group: &MediaListGroup,
	matched: Option<&HashMap<i32, Vec<LocalEpisode>>>,
) {
	TableBuilder::new(ui)
		.striped(true)
		.sense(egui::Sense::click())
		.cell_layout(egui::Layout::left_to_right(egui::Align::Center))
		.column(Column::remainder().clip(true).at_least(200.0))
		.column(Column::auto())
		.column(Column::exact(50.0))
		.column(Column::exact(60.0))
		.column(Column::exact(100.0))
		.column(Column::exact(100.0))
		.header(20.0, header_row)
		.body(|body| {
			body.rows(20.0, group.entries.len(), |row| {
				let entry = &group.entries[row.index()];
				entry_row(row, entry, matched);
			});
		});
}

fn header_row(mut header: TableRow<'_, '_>) {
	header.col(|ui| {
		ui.strong("Anime title");
	});
	header.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.strong("Progress");
		});
	});
	header.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.strong("Score");
		});
	});
	header.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.strong("Type");
		});
	});
	header.col(|ui| {
		ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
			ui.strong("Season");
		});
	});
	header.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.strong("Last updated");
		});
	});
}

fn entry_row(
	mut row: TableRow<'_, '_>,
	entry: &MediaList,
	matched: Option<&HashMap<i32, Vec<LocalEpisode>>>,
) {
	let media = entry.media.as_ref();

	let title = media
		.and_then(|media| media.title.as_ref())
		.and_then(|title| title.romaji.as_deref())
		.unwrap_or("Unknown title");

	let progress = u32::try_from(entry.progress.unwrap_or(0)).unwrap_or(0);

	let total_episodes = media
		.and_then(|media| media.episodes)
		.and_then(|total| u32::try_from(total).ok())
		.filter(|&total| total > 0);

	let episodes_text = total_episodes.map_or_else(|| "?".to_owned(), |total| total.to_string());

	let score = entry
		.score
		.filter(|&score| score > 0.0)
		.map_or_else(|| "-".to_owned(), |score| score.to_string());

	let format = media
		.and_then(|media| media.format)
		.map_or_else(|| "-", |format| format_label(format));

	let season = media
		.and_then(|media| media.season)
		.map_or_else(|| "?", |season| season_label(season));

	let season_year = media
		.and_then(|media| media.season_year)
		.map_or_else(|| "?".to_owned(), |season_year| season_year.to_string());

	let updated_at = entry
		.updated_at
		.map_or_else(|| "?".to_owned(), updated_label);

	let local_episodes: &[LocalEpisode] = matched
		.and_then(|matched| matched.get(&media?.id))
		.map(Vec::as_slice)
		.unwrap_or_default();

	let next_episode = local_episodes
		.iter()
		.find(|episode| episode.parsed.episode == progress + 1);

	// Title
	row.col(|ui| {
		ui.label(title);
	});

	// Progress
	row.col(|ui| {
		ui.spacing_mut().item_spacing.x = 2.0;

		progress_bar(ui, progress, total_episodes, local_episodes);

		let size = egui::vec2(20.0, ui.available_height());
		ui.allocate_ui_with_layout(
			size,
			egui::Layout::right_to_left(egui::Align::Center),
			|ui| {
				ui.label(progress.to_string());
			},
		);
		ui.label("/");
		ui.allocate_ui_with_layout(
			size,
			egui::Layout::left_to_right(egui::Align::Center),
			|ui| {
				ui.label(episodes_text);
			},
		);
	});

	// Score
	row.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.label(score);
		});
	});

	// Type
	row.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.label(format);
		});
	});

	// Season
	row.col(|ui| {
		ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
			ui.label(format!("{season} {season_year}"));
		});
	});

	// Last updated
	row.col(|ui| {
		ui.centered_and_justified(|ui| {
			ui.label(updated_at);
		});
	});

	row.response().context_menu(|ui| {
		let play_next = ui.add_enabled(
			next_episode.is_some(),
			egui::Button::new("Play next episode"),
		);

		if play_next.clicked()
			&& let Some(episode) = next_episode
		{
			if let Err(err) = open::that_detached(&episode.path) {
				log::warn!("could not open {}: {err}", episode.path.display());
			}
			ui.close();
		}
	});
}

#[expect(
	clippy::cast_precision_loss,
	reason = "episode counts are far below 2^24"
)]
fn progress_bar(ui: &mut egui::Ui, progress: u32, total: Option<u32>, episodes: &[LocalEpisode]) {
	// Progress bar background
	let (rect, _response) = ui.allocate_exact_size(egui::vec2(150.0, 12.0), egui::Sense::hover());

	ui.painter()
		.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

	// Downloaded episodes strip background
	let episode_strip =
		egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 3.0), rect.max);

	ui.painter()
		.rect_filled(episode_strip, 0.0, ui.visuals().widgets.inactive.bg_fill);

	// Progress bar fill
	let fraction = match (progress, total) {
		(0, _) => 0.0,
		(_, Some(total)) => progress as f32 / total as f32,
		_ => 0.5,
	};

	let filled = egui::Rect::from_min_size(
		rect.min,
		egui::vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
	);

	ui.painter()
		.rect_filled(filled, 0.0, egui::Color32::from_rgb(96, 198, 83));

	// Downloaded episodes strip fill
	if let Some(total) = total {
		let episode_width = episode_strip.width() / total as f32;
		for episode in episodes {
			let episode_number = episode.parsed.episode;
			if !(1..=total).contains(&episode_number) {
				continue;
			}

			let left = episode_strip.left() + (episode_number - 1) as f32 * episode_width;
			let episode_fill_strip = egui::Rect::from_min_max(
				egui::pos2(left, episode_strip.top()),
				egui::pos2(left + episode_width, episode_strip.bottom()),
			);
			ui.painter().rect_filled(
				episode_fill_strip,
				0.0,
				egui::Color32::from_rgb(75, 153, 64),
			);
		}
	}
}

fn format_label(format: MediaFormat) -> &'static str {
	match format {
		MediaFormat::Tv => "TV",
		MediaFormat::TvShort => "TV Short",
		MediaFormat::Movie => "Movie",
		MediaFormat::Special => "Special",
		MediaFormat::Ova => "OVA",
		MediaFormat::Ona => "ONA",
		MediaFormat::Music => "Music",
		MediaFormat::Other => "?",
	}
}

fn season_label(season: MediaSeason) -> &'static str {
	match season {
		MediaSeason::Winter => "Winter",
		MediaSeason::Spring => "Spring",
		MediaSeason::Summer => "Summer",
		MediaSeason::Fall => "Fall",
	}
}

fn updated_label(updated_at: i32) -> String {
	const MINUTE: i64 = 60;
	const HOUR: i64 = 60 * MINUTE;
	const DAY: i64 = 24 * HOUR;

	let Ok(timestamp) = jiff::Timestamp::from_second(i64::from(updated_at)) else {
		return "-".to_owned();
	};
	let seconds = jiff::Timestamp::now().duration_since(timestamp).as_secs();

	match seconds {
		s if s < MINUTE => "just now".to_owned(),
		s if s < HOUR => ago(s / MINUTE, "minute"),
		s if s < DAY => ago(s / HOUR, "hour"),
		s if s < 21 * DAY => ago(s / DAY, "day"),
		_ => {
			let date = timestamp.to_zoned(jiff::tz::TimeZone::system()).date();
			if date.year() == jiff::Zoned::now().year() {
				date.strftime("%B %-d").to_string()
			} else {
				date.to_string()
			}
		}
	}
}

fn ago(n: i64, unit: &str) -> String {
	let plural = if n == 1 { "" } else { "s" };
	format!("{n} {unit}{plural} ago")
}
