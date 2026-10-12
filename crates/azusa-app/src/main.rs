// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

#[derive(Default)]
struct AzusaApp {
    clicks: u32,
}

impl eframe::App for AzusaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Azusa");
            ui.label(format!("Clicks: {}", self.clicks));

            if ui.button("Click me").clicked() {
                self.clicks += 1;
            }
        });
    }
}

fn main() -> eframe::Result {
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
