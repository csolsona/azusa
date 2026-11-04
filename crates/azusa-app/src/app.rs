use std::{
	collections::HashMap,
	env, io,
	sync::{Arc, mpsc},
};

use azusa_core::{
	anilist::{self, AccessToken, MediaList, MediaListGroup, MediaListStatus, Session},
	library::{self, LocalEpisode, MatchedFile},
};
use eframe::egui::{self, Align, Atom, Layout, menu::MenuButton};

use crate::{
	playback,
	sync::{ProgressSync, SyncFailure, fetch_lists},
	ui::anime_table::{self, ProgressChange},
};

#[derive(Default)]
pub struct AzusaApp {
	auth: AuthState,
	library: LibraryState,
}

fn apply_default_styles(ctx: &egui::Context) {
	ctx.all_styles_mut(|style| {
		let widgets = &mut style.visuals.widgets;
		for state in [
			&mut widgets.noninteractive,
			&mut widgets.inactive,
			&mut widgets.hovered,
			&mut widgets.active,
			&mut widgets.open,
		] {
			state.corner_radius = egui::CornerRadius::ZERO;
		}
	});
}

impl AzusaApp {
	pub fn new(ctx: &egui::Context) -> Self {
		let mut app = Self::default();
		apply_default_styles(ctx);
		app.start_scan(ctx);
		match anilist::load_token() {
			Ok(Some(token)) => app.spawn_session(ctx, move || Ok(token)),
			Ok(None) => {}
			Err(err) => app.auth = AuthState::Failed(err),
		}
		app
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

		if let LibraryState::Scanning(rx) = &self.library
			&& let Ok(result) = rx.try_recv()
		{
			self.library = match result {
				Ok(episodes) => LibraryState::Scanned(episodes),
				Err(err) => {
					log::warn!("library scan failed: {err}");
					LibraryState::Failed(err)
				}
			};
		}

		if let AuthState::LoggedIn(account) = &mut self.auth {
			account.poll();
		}

		if let AuthState::LoggedIn(Account { lists, .. }) = &self.auth
			&& let LibraryState::Scanned(episodes) = &mut self.library
		{
			let episodes = std::mem::take(episodes);
			let matched = library::match_library(lists, episodes);

			log::info!(
				"{} episodes matched across {} anime",
				matched.values().map(Vec::len).sum::<usize>(),
				matched.len(),
			);

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

fn chevron() -> Atom<'static> {
	Atom::paint(egui::vec2(8.0, 4.0), |ui, args| {
		// Text looks centered around its x-height, below the center of the line box.
		let r = args.rect.translate(egui::vec2(0.0, 1.0));
		let stroke = egui::Stroke::new(1.5, args.fallback_text_color);
		ui.painter()
			.line(vec![r.left_top(), r.center_bottom(), r.right_top()], stroke);
	})
}

fn bar_frame(ui: &egui::Ui) -> egui::Frame {
	let visuals = ui.visuals();
	let fill = if visuals.dark_mode {
		egui::Color32::from_gray(36)
	} else {
		visuals.panel_fill
	};
	egui::Frame::side_top_panel(ui.style()).fill(fill)
}

impl eframe::App for AzusaApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		self.poll(ui.ctx());

		if let AuthState::LoggedIn(account) = &self.auth {
			egui::Panel::top("top-panel")
				.frame(bar_frame(ui))
				.show(ui, |ui| {
					egui::MenuBar::new().ui(ui, |ui| {
						if ui.button("Synchronize list").clicked() {
							log::info!("Sync list");
						}

						if ui.button("Scan available episodes").clicked() {
							log::info!("Scan episodes");
						}

						ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
							let account_name = account.session.viewer.name.as_str();
							let button = egui::Button::new((account_name, chevron())).gap(5.0);
							MenuButton::from_button(button).ui(ui, |ui| {
								let text = egui::RichText::new("AniList profile ↗")
									.underline()
									.color(ui.visuals().hyperlink_color);
								let url = format!(
									"https://anilist.co/user/{}",
									account.session.viewer.id
								);

								if ui.button(text).clicked() {
									ui.open_url(egui::OpenUrl::new_tab(url));
								}
								ui.separator();
								ui.add_enabled(false, egui::Button::new("Manage accounts"));
							})
						});
					});
				});
		}

		if let AuthState::LoggedIn(account) = &mut self.auth {
			egui::Panel::bottom("status-bar")
				.frame(bar_frame(ui))
				.resizable(false)
				.exact_size(24.0)
				.show(ui, |ui| {
					let error = match (&account.error, &self.library) {
						(Some(err), _) => Some(err.to_string()),
						(None, LibraryState::Failed(err)) => {
							Some(format!("Library scan failed: {err}"))
						}
						_ => None,
					};

					if let Some(error) = error {
						let text = egui::RichText::new(error).color(ui.visuals().error_fg_color);
						ui.add(egui::Label::new(text).truncate());
					}
				});

			egui::Panel::left("sidebar")
				.frame(bar_frame(ui))
				.resizable(false)
				.exact_size(120.0)
				.show(ui, |ui| {
					ui.selectable_value(&mut account.view, View::NowPlaying, "Now playing");
					ui.selectable_value(&mut account.view, View::AnimeList, "Anime list");
				});
		}

		let list_open =
			matches!(&self.auth, AuthState::LoggedIn(account) if account.view == View::AnimeList);
		let margin = if list_open { 0 } else { 8 };
		let frame = egui::Frame::central_panel(ui.style()).inner_margin(margin);

		egui::CentralPanel::default()
			.frame(frame)
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
	Scanning(mpsc::Receiver<io::Result<Vec<LocalEpisode>>>),
	Scanned(Vec<LocalEpisode>),
	Matched(HashMap<i32, Vec<LocalEpisode>>),
	Failed(io::Error),
}

#[derive(PartialEq)]
enum View {
	NowPlaying,
	AnimeList,
}

struct Account {
	session: Arc<Session>,
	lists: Vec<MediaListGroup>,
	view: View,
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
			view: View::AnimeList,
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

	fn playing_entry(&self) -> Option<&MediaList> {
		let playing = self.playing.as_ref()?;
		self.lists
			.iter()
			.flat_map(|group| group.entries.iter())
			.find(|entry| {
				entry
					.media
					.as_ref()
					.is_some_and(|media| media.id == playing.media_id)
			})
	}

	fn anime_list_view(&mut self, ui: &mut egui::Ui, library: &LibraryState) {
		let matched = match library {
			LibraryState::Matched(matched) => Some(matched),
			_ => None,
		};

		let row = ui.horizontal(|ui| {
			ui.spacing_mut().item_spacing.x = 0.0;
			let mut selected_rect = None;

			for status in MediaListStatus::ALL {
				let selected = self.selected_status == status;
				let response = status_tab(ui, status_label(status), selected);

				if selected {
					selected_rect = Some(response.rect);
				}
				if response.clicked() {
					self.selected_status = status;
				}
			}

			selected_rect
		});

		if let Some(tab) = row.inner {
			let border = ui.visuals().widgets.noninteractive.bg_stroke;
			let area = ui.clip_rect();
			let painter = ui.painter();

			if tab.left() > area.left() {
				painter.hline(area.left()..=tab.left(), tab.bottom(), border);
			}
			painter.hline(tab.right()..=area.right(), tab.bottom(), border);
		}

		let anime_list = self
			.lists
			.iter()
			.find(|group| group.status == Some(self.selected_status));

		if let Some(group) = anime_list {
			ui.style_mut().interaction.selectable_labels = false;

			if let Some(change) = anime_table::show(ui, group, matched) {
				self.set_progress(change);
			}
		} else {
			ui.label("No lists");
		}
	}

	fn now_playing_view(&mut self, ui: &mut egui::Ui) {
		if let Some(playing) = &self.playing
			&& let Some(entry) = self.playing_entry()
		{
			let title = entry
				.media
				.as_ref()
				.and_then(|media| media.title.as_ref())
				.and_then(|title| title.romaji.as_deref())
				.unwrap_or("Unknown title");

			ui.heading(title);
			ui.label(format!("Now playing: Episode {}", playing.episode));

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

			if let Some(change) = change {
				self.set_progress(change);
			}
		} else {
			ui.label("Nothing playing");
		}
	}
}

fn status_tab(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
	let response = ui
		.scope(|ui| {
			let visuals = ui.visuals_mut();
			let (rest, hover) = if selected {
				(visuals.panel_fill, visuals.panel_fill)
			} else {
				(
					visuals.faint_bg_color,
					visuals.selection.bg_fill.gamma_multiply(0.4),
				)
			};

			let widgets = &mut visuals.widgets;
			widgets.inactive.weak_bg_fill = rest;
			for state in [&mut widgets.hovered, &mut widgets.active] {
				state.weak_bg_fill = hover;
				state.bg_stroke = egui::Stroke::NONE;
			}

			ui.add(egui::Button::new(label).min_size(egui::vec2(0.0, 22.0)))
		})
		.inner;

	if selected {
		let r = response.rect;
		let border = ui.visuals().widgets.noninteractive.bg_stroke;
		let emphasis = egui::Stroke::new(2.0, ui.visuals().strong_text_color());
		let painter = ui.painter();
		if r.left() > ui.clip_rect().left() {
			painter.vline(r.left(), r.y_range(), border);
		}
		painter.vline(r.right(), r.y_range(), border);
		painter.hline(r.x_range(), r.top(), emphasis);
	}

	response
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
