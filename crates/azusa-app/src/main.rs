// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;

use crate::app::AzusaApp;

fn main() -> eframe::Result {
	env_logger::init();

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 320.0]),
		..Default::default()
	};

	eframe::run_native(
		"Azusa",
		options,
		Box::new(|_cc| Ok(Box::new(AzusaApp::default()))),
	)
}
