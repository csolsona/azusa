use std::sync::mpsc;

use azusa_core::anilist::{self, Session};
use eframe::egui;

#[derive(Default)]
enum AuthState {
	#[default]
	LoggedOut,
	Waiting(mpsc::Receiver<Result<Session, anilist::Error>>),
	LoggedIn(Session),
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
			let _ = tx.send(server.wait_for_token().and_then(Session::start));
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
				Ok(session) => AuthState::LoggedIn(session),
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
				AuthState::LoggedIn(session) => {
					ui.label(format!("Welcome, {}", session.viewer.name));
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
