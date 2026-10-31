// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod sync;
mod ui;

use crate::app::AzusaApp;
use eframe::egui;

fn main() -> eframe::Result {
	env_logger::init();

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([930.0, 620.0]),
		..Default::default()
	};

	eframe::run_native(
		"Azusa",
		options,
		Box::new(|cc| Ok(Box::new(AzusaApp::new(&cc.egui_ctx)))),
	)
}
