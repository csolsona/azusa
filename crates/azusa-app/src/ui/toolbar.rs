use azusa_core::anilist::Viewer;
use eframe::egui;

use super::style;

pub fn show(ui: &mut egui::Ui, viewer: &Viewer) {
	egui::Panel::top("toolbar")
		.frame(style::bar_frame(ui))
		.show(ui, |ui| {
			egui::MenuBar::new().ui(ui, |ui| {
				if ui.button("Synchronize list").clicked() {
					log::info!("Sync list");
				}

				if ui.button("Scan available episodes").clicked() {
					log::info!("Scan episodes");
				}

				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					let account_name = viewer.name.as_str();
					let button = egui::Button::new((account_name, chevron())).gap(5.0);
					egui::menu::MenuButton::from_button(button).ui(ui, |ui| {
						let text = egui::RichText::new("AniList profile ↗")
							.underline()
							.color(ui.visuals().hyperlink_color);
						let url = format!("https://anilist.co/user/{}", viewer.id);

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

fn chevron() -> egui::Atom<'static> {
	egui::Atom::paint(egui::vec2(8.0, 4.0), |ui, args| {
		// Text looks centered around its x-height, below the center of the line box.
		let rect = args.rect.translate(egui::vec2(0.0, 1.0));
		let stroke = egui::Stroke::new(1.5, args.fallback_text_color);
		ui.painter().line(
			vec![rect.left_top(), rect.center_bottom(), rect.right_top()],
			stroke,
		);
	})
}
