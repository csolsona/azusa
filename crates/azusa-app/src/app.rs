use std::{
	collections::HashMap,
	env, io,
	sync::{Arc, mpsc},
};

use azusa_core::{
	anilist::{self, AccessToken, MediaListGroup, MediaListStatus, Session},
	library::{self, LocalEpisode, MatchedFile},
};
use eframe::egui;

use crate::{
	playback,
	sync::{ProgressSync, SyncFailure, fetch_lists},
	ui::anime_table::{self, ProgressChange},
};

#[derive(Default)]
enum AuthState {
	#[default]
	LoggedOut,
	Waiting(mpsc::Receiver<Result<(Session, Vec<MediaListGroup>), anilist::Error>>),
	LoggedIn(Account),
	Failed(anilist::Error),
}

struct Account {
	session: Arc<Session>,
	lists: Vec<MediaListGroup>,
	selected_status: MediaListStatus,
	sync: ProgressSync,
	error: Option<anilist::Error>,
	playback: mpsc::Receiver<Option<String>>,
	playing: Option<MatchedFile>,
}

impl Account {
	fn new(ctx: &egui::Context, session: Session, lists: Vec<MediaListGroup>) -> Self {
		let session = Arc::new(session);
		let sync = ProgressSync::spawn(ctx, Arc::clone(&session));

		Self {
			session,
			lists,
			selected_status: MediaListStatus::Current,
			sync,
			error: None,
			playback: playback::spawn_monitor(ctx),
			playing: None,
		}
	}

	fn poll(&mut self) {
		if let Some(SyncFailure { error, lists }) = self.sync.poll_error() {
			log::warn!("progress update failed: {error}");
			self.error = Some(error);

			if let Some(lists) = lists {
				self.lists = lists;
			}
		}

		if let Ok(file_name) = self.playback.try_recv() {
			self.playing = file_name.and_then(|name| library::match_file(&self.lists, &name));
			log::info!("now playing: {:?}", self.playing);
		}
	}

	fn set_progress(&mut self, change: ProgressChange) {
		self.error = None;

		let Ok(progress) = i32::try_from(change.progress) else {
			return;
		};
		if let Some(entry) = self
			.lists
			.iter_mut()
			.flat_map(|group| group.entries.iter_mut())
			.find(|entry| entry.id == change.entry_id)
		{
			entry.progress = Some(progress);
		}
		self.sync.send(change);
	}
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

				let lists = fetch_lists(&session)?;

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
				Ok((session, lists)) => AuthState::LoggedIn(Account::new(ui.ctx(), session, lists)),
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

		if let AuthState::LoggedIn(account) = &mut self.auth {
			account.poll();
		}

		if let AuthState::LoggedIn(Account { lists, .. }) = &self.auth
			&& let LibraryState::Scanned(episodes) = &mut self.library
		{
			let episodes = std::mem::take(episodes);
			self.library = LibraryState::Matched(library::match_library(lists, episodes));
		}

		egui::Panel::top("top-panel").show(ui, |ui| {
			egui::MenuBar::new().ui(ui, |ui| {
				ui.menu_button("File", |ui| {
					if ui.button("Test").clicked() {
						log::info!("menu placeholder clicked");
					}
				});
			});
		});

		egui::CentralPanel::default().show(ui, |ui| {
			ui.heading("Azusa");

			library_status(ui, &self.library);

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
				AuthState::LoggedIn(account) => {
					ui.label(format!("Welcome, {}", account.session.viewer.name));

					if let Some(err) = &account.error {
						ui.colored_label(ui.visuals().error_fg_color, err.to_string());
					}

					ui.horizontal(|ui| {
						for status in MediaListStatus::ALL {
							ui.selectable_value(
								&mut account.selected_status,
								status,
								status_label(status),
							);
						}
					});

					let anime_list = account
						.lists
						.iter()
						.find(|group| group.status == Some(account.selected_status));

					if let Some(group) = anime_list {
						ui.style_mut().interaction.selectable_labels = false;

						if let Some(change) = anime_table::show(ui, group, matched) {
							account.set_progress(change);
						}
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

fn library_status(ui: &mut egui::Ui, library: &LibraryState) {
	match library {
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
