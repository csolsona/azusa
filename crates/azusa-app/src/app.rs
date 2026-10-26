use std::{collections::HashMap, env, io, sync::mpsc};

use azusa_core::{
	anilist::{
		self, AccessToken, MediaFormat, MediaListGroup, MediaListStatus, MediaSeason, Session,
	},
	library::{self, LocalEpisode},
};
use eframe::egui::{self, Layout};
use egui_extras::{Column, TableBuilder};

#[derive(Default)]
enum AuthState {
	#[default]
	LoggedOut,
	Waiting(mpsc::Receiver<Result<(Session, Vec<MediaListGroup>), anilist::Error>>),
	LoggedIn {
		session: Session,
		selected_status: MediaListStatus,
		lists: Vec<MediaListGroup>,
	},
	Failed(anilist::Error),
}

#[derive(Default)]
enum LibraryState {
	#[default]
	NotScanned,
	Scanning(mpsc::Receiver<io::Result<Vec<LocalEpisode>>>),
	Scanned(Vec<LocalEpisode>),
	Matched(HashMap<i32, Vec<LocalEpisode>>),
	Failed(io::Error),
}

#[derive(Default)]
pub struct AzusaApp {
	auth: AuthState,
	library: LibraryState,
}

impl AzusaApp {
	pub fn new(ctx: &egui::Context) -> Self {
		let mut app = Self::default();
		app.start_scan(ctx);
		match anilist::load_token() {
			Ok(Some(token)) => app.spawn_session(ctx, move || Ok(token)),
			Ok(None) => {}
			Err(err) => app.auth = AuthState::Failed(err),
		}
		app
	}

	/// Spawns the loopback server in a background thread and opens the browser.
	fn start_login(&mut self, ctx: &egui::Context) {
		let server = match anilist::LoginServer::bind() {
			Ok(server) => server,
			Err(err) => {
				self.auth = AuthState::Failed(err);
				return;
			}
		};
		self.spawn_session(ctx, move || server.wait_for_token());
		ctx.open_url(egui::OpenUrl::new_tab(anilist::authorize_url()));
	}

	/// Fetches the session and lists in a background thread, using the token from `get_token`.
	fn spawn_session(
		&mut self,
		ctx: &egui::Context,
		get_token: impl FnOnce() -> Result<AccessToken, anilist::Error> + Send + 'static,
	) {
		let (tx, rx) = mpsc::channel();
		let thread_ctx = ctx.clone();
		std::thread::spawn(move || {
			let _ = tx.send(get_token().and_then(Session::start).and_then(|session| {
				anilist::save_token(&session.token)?;
				let lists = session.anime_lists()?;
				Ok((session, lists))
			}));
			thread_ctx.request_repaint();
		});
		self.auth = AuthState::Waiting(rx);
	}

	/// Scans the library directory in a background thread.
	fn start_scan(&mut self, ctx: &egui::Context) {
		let (tx, rx) = mpsc::channel();
		let thread_ctx = ctx.clone();

		// TODO: hardcoded shortcut until settings menu exists
		let dir = env::home_dir()
			.expect("home directory should exist")
			.join("downloads");

		std::thread::spawn(move || {
			let _ = tx.send(library::scan(&dir));
			thread_ctx.request_repaint();
		});
		self.library = LibraryState::Scanning(rx);
	}
}

impl eframe::App for AzusaApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		if let AuthState::Waiting(rx) = &self.auth
			&& let Ok(result) = rx.try_recv()
		{
			self.auth = match result {
				Ok((session, lists)) => AuthState::LoggedIn {
					session,
					selected_status: MediaListStatus::Current,
					lists,
				},
				Err(err) => AuthState::Failed(err),
			};
		}

		if let LibraryState::Scanning(rx) = &self.library
			&& let Ok(result) = rx.try_recv()
		{
			self.library = match result {
				Ok(episodes) => LibraryState::Scanned(episodes),
				Err(err) => LibraryState::Failed(err),
			};
		}

		if let AuthState::LoggedIn { lists, .. } = &self.auth
			&& let LibraryState::Scanned(episodes) = &mut self.library
		{
			let episodes = std::mem::take(episodes);
			self.library = LibraryState::Matched(library::match_library(lists, episodes));
		}

		egui::Panel::top("top-panel").show(ui, |ui| {
			egui::MenuBar::new().ui(ui, |ui| {
				ui.menu_button("File", |ui| {
					ui.button("Test");
				});
			});
		});

		egui::CentralPanel::default().show(ui, |ui| {
			ui.heading("Azusa");

			match &self.library {
				LibraryState::NotScanned => {
					ui.label("Not scanned");
				}
				LibraryState::Scanning(_) => {
					ui.horizontal(|ui| {
						ui.spinner();
						ui.label("Scanning…");
					});
				}
				LibraryState::Scanned(episodes) => {
					let count = episodes.len();
					ui.label(format!("{count} episodes scanned"));
				}
				LibraryState::Matched(episodes) => {
					let count = episodes.len();
					ui.label(format!("{count} episodes matched"));
				}
				LibraryState::Failed(err) => {
					ui.colored_label(ui.visuals().error_fg_color, err.to_string());
				}
			}

			let matched = match &self.library {
				LibraryState::Matched(matched) => Some(matched),
				_ => None,
			};

			match &mut self.auth {
				AuthState::LoggedOut => {
					if ui.button("AniList login").clicked() {
						self.start_login(ui.ctx());
					}
				}
				AuthState::Waiting(_) => {
					ui.horizontal(|ui| {
						ui.spinner();
						ui.label("Waiting for the browser…");
					});
				}
				AuthState::LoggedIn {
					session,
					selected_status,
					lists,
				} => {
					ui.label(format!("Welcome, {}", session.viewer.name));

					ui.horizontal(|ui| {
						for status in MediaListStatus::ALL {
							ui.selectable_value(selected_status, status, status_label(status));
						}
					});

					let anime_list = lists
						.iter()
						.find(|group| group.status == Some(*selected_status));

					if let Some(group) = anime_list {
						TableBuilder::new(ui)
							.striped(true)
							.cell_layout(egui::Layout::left_to_right(egui::Align::Center))
							.column(Column::remainder().clip(true).at_least(200.0))
							.column(Column::auto())
							.column(Column::exact(50.0))
							.column(Column::exact(60.0))
							.column(Column::exact(100.0))
							.column(Column::exact(100.0))
							.header(20.0, |mut header| {
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
									ui.with_layout(
										Layout::right_to_left(egui::Align::Center),
										|ui| {
											ui.strong("Season");
										},
									);
								});
								header.col(|ui| {
									ui.centered_and_justified(|ui| {
										ui.strong("Last updated");
									});
								});
							})
							.body(|body| {
								body.rows(20.0, group.entries.len(), |mut row| {
									let entry = &group.entries[row.index()];

									let media = entry.media.as_ref();

									let title = media
										.and_then(|media| media.title.as_ref())
										.and_then(|title| title.romaji.as_deref())
										.unwrap_or("Unknown title");

									let progress = entry.progress.unwrap_or(0);

									let total_episodes = media.and_then(|media| media.episodes);

									let episodes_text = total_episodes
										.map_or_else(|| "?".to_owned(), |total| total.to_string());

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

									let season_year =
										media.and_then(|media| media.season_year).map_or_else(
											|| "?".to_owned(),
											|season_year| season_year.to_string(),
										);

									let updated_at = entry
										.updated_at
										.map_or_else(|| "?".to_owned(), updated_label);

									let local_episodes: &[LocalEpisode] = matched
										.and_then(|matched| matched.get(&media?.id))
										.map(Vec::as_slice)
										.unwrap_or_default();

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
										ui.with_layout(
											Layout::right_to_left(egui::Align::Center),
											|ui| {
												ui.label(format!("{season} {season_year}"));
											},
										);
									});

									// Last updated
									row.col(|ui| {
										ui.centered_and_justified(|ui| {
											ui.label(updated_at);
										});
									});
								});
							});
					} else {
						ui.label("No lists");
					}
				}
				AuthState::Failed(err) => {
					ui.colored_label(ui.visuals().error_fg_color, err.to_string());
					if ui.button("Retry").clicked() {
						self.start_login(ui.ctx());
					}
				}
			}
		});
	}
}

#[expect(
	clippy::cast_precision_loss,
	reason = "episode counts are far below 2^24"
)]
fn progress_bar(ui: &mut egui::Ui, progress: i32, total: Option<i32>, episodes: &[LocalEpisode]) {
	let progress = u32::try_from(progress).unwrap_or(0);
	let total = total
		.and_then(|total| u32::try_from(total).ok())
		.filter(|&total| total > 0);

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

fn status_label(status: MediaListStatus) -> &'static str {
	match status {
		MediaListStatus::Current => "Watching",
		MediaListStatus::Completed => "Completed",
		MediaListStatus::Paused => "Paused",
		MediaListStatus::Dropped => "Dropped",
		MediaListStatus::Planning => "Planning",
		MediaListStatus::Repeating => "Rewatching",
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
