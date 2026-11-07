// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod logging;
mod playback;
mod sync;
mod ui;

use crate::app::AzusaApp;
use eframe::egui;

fn main() -> eframe::Result {
	logging::init();

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([1056.0, 660.0]),
		..Default::default()
	};

	eframe::run_native(
		"Azusa",
		options,
		Box::new(|cc| Ok(Box::new(AzusaApp::new(cc)))),
	)
}
