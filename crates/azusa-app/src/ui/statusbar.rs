use eframe::egui;

use super::style;

pub enum Status {
	Scanning,
	Scanned { episodes: usize },
	Syncing,
	Synced,
	Error(String),
}

pub fn show(ui: &mut egui::Ui, status: Option<&Status>) {
	egui::Panel::bottom("statusbar")
		.frame(style::bar_frame(ui))
		.resizable(false)
		.exact_size(24.0)
		.show(ui, |ui| {
			ui.horizontal_centered(|ui| match status {
				Some(Status::Error(error)) => {
					let text = egui::RichText::new(error).color(ui.visuals().error_fg_color);
					ui.add(egui::Label::new(text).truncate());
				}
				Some(Status::Scanning) => {
					ui.weak("Scanning library…");
				}
				Some(Status::Scanned { episodes }) => {
					ui.weak(format!("{episodes} episodes found"));
				}
				Some(Status::Syncing) => {
					ui.weak("Syncing lists…");
				}
				Some(Status::Synced) => {
					ui.weak("Lists synced");
				}
				None => {}
			});
		});
}
