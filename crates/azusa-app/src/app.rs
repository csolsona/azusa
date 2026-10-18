use std::sync::mpsc;

use azusa_core::anilist::{self, MediaListGroup, MediaListStatus, Session};
use eframe::egui;
use egui_extras::{Column, TableBuilder};

#[derive(Default)]
enum AuthState {
	#[default]
	LoggedOut,
	Waiting(mpsc::Receiver<Result<(Session, Vec<MediaListGroup>), anilist::Error>>),
	LoggedIn {
		session: Session,
		lists: Vec<MediaListGroup>,
	},
	Failed(anilist::Error),
}

#[derive(Default)]
pub struct AzusaApp {
	auth: AuthState,
}

impl AzusaApp {
	/// Spawns the loopback server in a background thread and opens the browser.
	fn start_login(&mut self, ctx: &egui::Context) {
		let server = match anilist::LoginServer::bind() {
			Ok(server) => server,
			Err(err) => {
				self.auth = AuthState::Failed(err);
				return;
			}
		};

		let (tx, rx) = mpsc::channel();
		let thread_ctx = ctx.clone();
		std::thread::spawn(move || {
			let _ = tx.send(
				server
					.wait_for_token()
					.and_then(Session::start)
					.and_then(|session| {
						let lists = session.anime_lists()?;
						Ok((session, lists))
					}),
			);
			thread_ctx.request_repaint();
		});

		ctx.open_url(egui::OpenUrl::new_tab(anilist::authorize_url()));
		self.auth = AuthState::Waiting(rx);
	}
}

impl eframe::App for AzusaApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		if let AuthState::Waiting(rx) = &self.auth
			&& let Ok(result) = rx.try_recv()
		{
			self.auth = match result {
				Ok((session, lists)) => AuthState::LoggedIn { session, lists },
				Err(err) => AuthState::Failed(err),
			};
		}

		egui::CentralPanel::default().show(ui, |ui| {
			ui.heading("Azusa");

			match &self.auth {
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
				AuthState::LoggedIn { session, lists } => {
					ui.label(format!("Welcome, {}", session.viewer.name));

					let anime_list = lists
						.iter()
						.find(|group| matches!(group.status, Some(MediaListStatus::Current)));

					if let Some(group) = anime_list {
						TableBuilder::new(ui)
							.striped(true)
							.cell_layout(egui::Layout::left_to_right(egui::Align::Center))
							.column(Column::remainder())
							.column(Column::auto().at_least(50.0))
							.column(Column::initial(50.0).at_least(50.0))
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
							})
							.body(|body| {
								body.rows(20.0, group.entries.len(), |mut row| {
									let entry = &group.entries[row.index()];

									let title = entry
										.media
										.as_ref()
										.and_then(|media| media.title.as_ref())
										.and_then(|title| title.romaji.as_deref())
										.unwrap_or("Unknown title");

									let progress = entry.progress.unwrap_or(0);

									let total_episodes =
										entry.media.as_ref().and_then(|media| media.episodes);

									let episodes_text = total_episodes
										.map_or_else(|| "?".to_owned(), |total| total.to_string());

									let score = entry
										.score
										.filter(|&score| score > 0.0)
										.map_or_else(|| "-".to_owned(), |score| score.to_string());

									#[expect(
										clippy::cast_precision_loss,
										reason = "episode counts are far below 2^24"
									)]
									let fraction = match (progress, total_episodes) {
										(0, _) => 0.0,
										(_, Some(total)) if total > 0 => {
											progress as f32 / total as f32
										}
										_ => 0.5,
									};

									// Title
									row.col(|ui| {
										ui.label(title);
									});

									// Progress
									row.col(|ui| {
										ui.spacing_mut().item_spacing.x = 2.0;
										ui.add(
											egui::ProgressBar::new(fraction)
												.desired_width(150.0)
												.corner_radius(0),
										);

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
