use eframe::egui;

use super::style;

pub fn show(ui: &mut egui::Ui, error: Option<String>) {
	egui::Panel::bottom("statusbar")
		.frame(style::bar_frame(ui))
		.resizable(false)
		.exact_size(24.0)
		.show(ui, |ui| {
			if let Some(error) = error {
				let text = egui::RichText::new(error).color(ui.visuals().error_fg_color);
				ui.add(egui::Label::new(text).truncate());
			}
		});
}
