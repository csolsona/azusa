use std::{
	collections::HashMap,
	sync::{Arc, mpsc},
	time::Instant,
};

use azusa_core::{
	anilist::{self, AccessToken, MediaList, MediaListGroup, MediaListStatus, Session},
	library::{self, LocalEpisode},
};
use eframe::egui;

use crate::{
	config::{Config, ProgressUpdate},
	playback::{self, NowPlaying, watched_enough},
	sync::{ListSync, ProgressChange, SyncEvent, SyncFailure, fetch_lists},
	ui::{
		anime_list, now_playing,
		settings::Settings,
		sidebar::{self, View},
		statusbar::{self, Status},
		style, toolbar,
	},
};

#[derive(Default)]
pub struct AzusaApp {
	auth: AuthState,
	/// Kept apart from `library` so the previous results stay visible while rescanning.
	scan: Option<mpsc::Receiver<Vec<LocalEpisode>>>,
	library: LibraryState,
	settings: Settings,
	config: Config,
	status: Option<Status>,
}

impl AzusaApp {
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		let ctx = &cc.egui_ctx;
		let mut app = Self::default();

		if let Some(storage) = cc.storage
			&& let Some(config) = eframe::get_value(storage, eframe::APP_KEY)
		{
			app.config = config;
		}

		style::apply_default_styles(ctx);
		app.start_scan(ctx);

		match anilist::load_token() {
			Ok(Some(token)) => app.spawn_session(ctx, move || Ok(token)),
			Ok(None) => {}
			Err(err) => app.auth = AuthState::Failed(err),
		}
		app
	}

	fn set_status(&mut self, status: Status) {
		match &status {
			Status::Error(error) => log::warn!("{error}"),
			other => log::info!("{other:?}"),
		}
		self.status = Some(status);
	}

	/// Applies the results of background work to the app state.
	fn poll(&mut self, ctx: &egui::Context) {
		if let AuthState::Waiting(rx) = &self.auth
			&& let Ok(result) = rx.try_recv()
		{
			self.auth = match result {
				Ok((session, lists)) => AuthState::LoggedIn(Account::new(ctx, session, lists)),
				Err(err) => AuthState::Failed(err),
			};
		}

		if let Some(rx) = &self.scan
			&& let Ok(episodes) = rx.try_recv()
		{
			self.library = LibraryState::Scanned(episodes);
			self.scan = None;
		}

		if let AuthState::LoggedIn(account) = &mut self.auth
			&& let Some(status) = account.poll(&self.config.playback.progress_update)
		{
			self.set_status(status);
		}

		if let AuthState::LoggedIn(Account { lists, .. }) = &self.auth
			&& let LibraryState::Scanned(episodes) = &mut self.library
		{
			let episodes = std::mem::take(episodes);
			let matched = library::match_library(lists, episodes);

			let total_episodes: usize = matched.values().map(Vec::len).sum();
			let total_anime: usize = matched.len();

			self.set_status(Status::Scanned {
				episodes: total_episodes,
			});

			log::info!("{total_episodes} episodes matched across {total_anime} anime");

			self.library = LibraryState::Matched(matched);
		}
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

	/// Scans the library folders in a background thread.
	fn start_scan(&mut self, ctx: &egui::Context) {
		self.set_status(Status::Scanning);

		let (tx, rx) = mpsc::channel();
		let thread_ctx = ctx.clone();
		let folders = self.config.library.folders.clone();

		std::thread::spawn(move || {
			let mut episodes = Vec::new();

			for folder in &folders {
				match library::scan(folder) {
					Ok(found) => episodes.extend(found),
					Err(err) => log::warn!("skipping folder {}: {err}", folder.display()),
				}
			}

			let _ = tx.send(episodes);
			thread_ctx.request_repaint();
		});
		self.scan = Some(rx);
	}
}

impl eframe::App for AzusaApp {
	fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
		self.poll(ui.ctx());

		let action = if let AuthState::LoggedIn(account) = &mut self.auth {
			let action = toolbar::show(ui, &account.session.viewer);

			statusbar::show(ui, self.status.as_ref());

			sidebar::show(ui, &mut account.view);

			action
		} else {
			None
		};

		match action {
			Some(toolbar::Action::Sync) => {
				if let AuthState::LoggedIn(account) = &self.auth {
					account.sync.reload();
					self.set_status(Status::Syncing);
				}
			}
			Some(toolbar::Action::Scan) => self.start_scan(ui.ctx()),
			Some(toolbar::Action::OpenSettings) => self.settings.open = true,
			None => {}
		}

		let list_open =
			matches!(&self.auth, AuthState::LoggedIn(account) if account.view == View::AnimeList);
		let margin = if list_open { 0 } else { 8 };
		let panel_frame = egui::Frame::central_panel(ui.style()).inner_margin(margin);

		egui::CentralPanel::default()
			.frame(panel_frame)
			.show(ui, |ui| match &mut self.auth {
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
				AuthState::LoggedIn(account) => match account.view {
					View::AnimeList => account.anime_list_view(ui, &self.library),
					View::NowPlaying => account.now_playing_view(ui),
				},
				AuthState::Failed(err) => {
					ui.colored_label(ui.visuals().error_fg_color, err.to_string());
					if ui.button("Retry").clicked() {
						self.start_login(ui.ctx());
					}
				}
			});

		if self.settings.show(ui, &mut self.config) {
			if let Some(storage) = frame.storage_mut() {
				self.save(storage);
				storage.flush();
			}

			self.start_scan(ui.ctx());
		}
	}

	fn save(&mut self, storage: &mut dyn eframe::Storage) {
		eframe::set_value(storage, eframe::APP_KEY, &self.config);
	}
}

#[derive(Default)]
enum AuthState {
	#[default]
	LoggedOut,
	Waiting(mpsc::Receiver<Result<(Session, Vec<MediaListGroup>), anilist::Error>>),
	LoggedIn(Account),
	Failed(anilist::Error),
}

#[derive(Default)]
enum LibraryState {
	#[default]
	NotScanned,
	Scanned(Vec<LocalEpisode>),
	Matched(HashMap<i32, Vec<LocalEpisode>>),
}

struct Account {
	session: Arc<Session>,
	lists: Vec<MediaListGroup>,
	view: View,
	selected_status: MediaListStatus,
	sync: ListSync,
	playback: mpsc::Receiver<Option<String>>,
	playing: Option<NowPlaying>,
}

impl Account {
	fn new(ctx: &egui::Context, session: Session, lists: Vec<MediaListGroup>) -> Self {
		let session = Arc::new(session);
		let sync = ListSync::spawn(ctx, Arc::clone(&session));

		Self {
			session,
			lists,
			view: View::AnimeList,
			selected_status: MediaListStatus::Current,
			sync,
			playback: playback::spawn_monitor(ctx),
			playing: None,
		}
	}

	fn poll(&mut self, progress_update_config: &ProgressUpdate) -> Option<Status> {
		let status = match self.sync.poll() {
			Some(SyncEvent::ProgressFailed(SyncFailure { error, lists })) => {
				if let Some(lists) = lists {
					self.lists = lists;
				}
				Some(Status::Error(format!("Progress update failed: {error}")))
			}
			Some(SyncEvent::Reloaded(Ok(lists))) => {
				self.lists = lists;
				Some(Status::Synced)
			}
			Some(SyncEvent::Reloaded(Err(error))) => {
				Some(Status::Error(format!("Sync failed: {error}")))
			}
			None => None,
		};

		if let Ok(file_name) = self.playback.try_recv() {
			if let ProgressUpdate::Auto(auto_update) = progress_update_config
				&& let Some(change) = self.finished_episode(auto_update.required_percent)
			{
				log::info!("auto progress update: {change:?}");
				self.set_progress(change);
			}

			match file_name.and_then(|name| library::match_file(&self.lists, &name)) {
				Some(file) => {
					self.playing = Some(NowPlaying {
						file,
						open: true,
						opened_at: Instant::now(),
					});
					self.view = View::NowPlaying;
				}
				None => {
					if let Some(playing) = &mut self.playing {
						playing.open = false;
					}
				}
			}
			log::info!("now playing: {:?}", self.playing);
		}

		status
	}

	fn set_progress(&mut self, change: ProgressChange) {
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

	fn playing_entry(&self) -> Option<&MediaList> {
		let playing = self.playing.as_ref()?;
		self.lists
			.iter()
			.flat_map(|group| group.entries.iter())
			.find(|entry| {
				entry
					.media
					.as_ref()
					.is_some_and(|media| media.id == playing.file.media_id)
			})
	}

	fn finished_episode(&self, required_percent: u8) -> Option<ProgressChange> {
		let playing = self.playing.as_ref().filter(|playing| playing.open)?;
		let entry = self.playing_entry()?;

		let episode_duration_minutes = entry.media.as_ref().and_then(|media| media.duration);

		let should_update = playing.is_next_episode(entry.progress)
			&& watched_enough(
				playing.opened_at.elapsed(),
				episode_duration_minutes,
				required_percent,
			);

		should_update.then_some(ProgressChange {
			entry_id: entry.id,
			progress: playing.file.episode,
		})
	}

	fn anime_list_view(&mut self, ui: &mut egui::Ui, library: &LibraryState) {
		let matched = match library {
			LibraryState::Matched(matched) => Some(matched),
			_ => None,
		};

		if let Some(change) = anime_list::show(ui, &self.lists, &mut self.selected_status, matched)
		{
			self.set_progress(change);
		}
	}

	fn now_playing_view(&mut self, ui: &mut egui::Ui) {
		if let Some(change) = now_playing::show(ui, self.playing.as_ref(), self.playing_entry()) {
			self.set_progress(change);
		}
	}
}
