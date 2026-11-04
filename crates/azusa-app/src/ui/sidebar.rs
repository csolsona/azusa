use eframe::egui;

use super::style;

#[derive(PartialEq)]
pub enum View {
	NowPlaying,
	AnimeList,
}

pub fn show(ui: &mut egui::Ui, view: &mut View) {
	egui::Panel::left("sidebar")
		.frame(style::bar_frame(ui))
		.resizable(false)
		.exact_size(120.0)
		.show(ui, |ui| {
			ui.selectable_value(view, View::NowPlaying, "Now playing");
			ui.selectable_value(view, View::AnimeList, "Anime list");
		});
}
