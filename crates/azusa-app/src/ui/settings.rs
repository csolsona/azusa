use eframe::egui;

use crate::{
	config::{AutoUpdateConfig, Config, LibraryConfig, PlaybackConfig, ProgressUpdate},
	ui::style,
};

#[derive(Default)]
pub struct Settings {
	pub open: bool,
	view: SettingsView,

	draft: Option<Config>,

	library_page: LibraryPage,
	playback_page: PlaybackPage,
}

#[derive(Default, PartialEq)]
enum SettingsView {
	Accounts,
	#[default]
	Library,
	Playback,
}

impl Settings {
	/// Returns true if the library folders changed.
	#[must_use]
	pub fn show(&mut self, ui: &mut egui::Ui, config: &mut Config) -> bool {
		if !self.open {
			return false;
		}

		ui.ctx().show_viewport_immediate(
			egui::ViewportId::from_hash_of("settings"),
			egui::ViewportBuilder::default()
				.with_title("Settings")
				.with_inner_size([630.0, 700.0]),
			|ui, _class| {
				let mut close = ui.input(|i| i.viewport().close_requested());
				let mut folders_changed = false;

				let draft = self.draft.get_or_insert_with(|| config.clone());

				egui::Panel::left("settings_navbar")
					.frame(style::bar_frame(ui))
					.resizable(false)
					.exact_size(120.0)
					.show(ui, |ui| {
						ui.selectable_value(&mut self.view, SettingsView::Accounts, "Accounts");
						ui.selectable_value(&mut self.view, SettingsView::Library, "Library");
						ui.selectable_value(&mut self.view, SettingsView::Playback, "Playback");
					});

				egui::Panel::bottom("settings_actions")
					.frame(egui::Frame::central_panel(ui.style()).inner_margin(8))
					.show_separator_line(false)
					.show(ui, |ui| {
						ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
							if ui.button("Cancel").clicked() {
								close = true;
							}

							if ui.button("Save").clicked() {
								folders_changed = draft.library != config.library;
								*config = draft.clone();

								close = true;
							}
						})
					});

				let frame = egui::Frame::central_panel(ui.style()).inner_margin(8);

				egui::CentralPanel::default()
					.frame(frame)
					.show(ui, |ui| match self.view {
						SettingsView::Accounts => {
							ui.heading("Accounts");
						}
						SettingsView::Library => {
							self.library_page.show(ui, &mut draft.library);
						}
						SettingsView::Playback => {
							self.playback_page.show(ui, &mut draft.playback);
						}
					});

				if close {
					self.draft = None;
					self.library_page = LibraryPage::default();
					self.playback_page = PlaybackPage::default();
					self.open = false;
				}

				folders_changed
			},
		)
	}
}

#[derive(Default)]
struct LibraryPage {
	selected: Option<usize>,
}

impl LibraryPage {
	fn show(&mut self, ui: &mut egui::Ui, library: &mut LibraryConfig) {
		ui.heading("Library");

		egui::Frame::group(ui.style())
			.inner_margin(0)
			.show(ui, |ui| {
				egui::ScrollArea::vertical()
					.max_height(120.0)
					.auto_shrink(false)
					.show(ui, |ui| {
						ui.spacing_mut().item_spacing.y = 0.0;
						ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
							for (i, path) in library.folders.iter().enumerate() {
								ui.selectable_value(
									&mut self.selected,
									Some(i),
									path.display().to_string(),
								);
							}
						});
					});
			});

		ui.horizontal(|ui| {
			if ui.button("Add").clicked()
				&& let Some(path) = rfd::FileDialog::new().pick_folder()
				&& !library.folders.contains(&path)
			{
				library.folders.push(path);
			}

			if ui
				.add_enabled(self.selected.is_some(), egui::Button::new("Remove"))
				.clicked() && let Some(i) = self.selected
			{
				library.folders.remove(i);
				self.selected = None;
			}
		});
	}
}

#[derive(Default)]
struct PlaybackPage {
	last_auto_update: AutoUpdateConfig,
}

impl PlaybackPage {
	fn show(&mut self, ui: &mut egui::Ui, playback: &mut PlaybackConfig) {
		ui.heading("Playback");

		ui.strong("List updates");
		egui::Frame::group(ui.style()).show(ui, |ui| {
			let mut auto_update = matches!(playback.progress_update, ProgressUpdate::Auto(_));
			if ui
				.checkbox(&mut auto_update, "Update progress automatically")
				.changed()
			{
				if auto_update {
					playback.progress_update = ProgressUpdate::Auto(self.last_auto_update.clone());
				} else {
					if let ProgressUpdate::Auto(config) = &playback.progress_update {
						self.last_auto_update = config.clone();
					}
					playback.progress_update = ProgressUpdate::Manual;
				}
			}

			let required_percent = match &mut playback.progress_update {
				ProgressUpdate::Auto(config) => &mut config.required_percent,
				ProgressUpdate::Manual => &mut self.last_auto_update.required_percent,
			};

			let text_color = ui.visuals().widgets.inactive.text_color();
			ui.add_enabled_ui(auto_update, |ui| {
				ui.indent("auto_update_options", |ui| {
					ui.horizontal(|ui| {
						ui.label(egui::RichText::new("after watching").color(text_color));
						ui.add(
							egui::DragValue::new(required_percent)
								.range(1..=100)
								.suffix("%"),
						);
						ui.label(egui::RichText::new("of an episode").color(text_color));
					});

					let mut wait_for_close = true;
					ui.add_enabled(
						false,
						egui::Checkbox::new(&mut wait_for_close, "Wait for media player to close"),
					)
					.on_disabled_hover_text("Not configurable yet");
				});
			});
		});
	}
}
