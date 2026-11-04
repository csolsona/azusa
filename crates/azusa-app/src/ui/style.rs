use eframe::egui;

pub fn apply_default_styles(ctx: &egui::Context) {
	ctx.all_styles_mut(|style| {
		let widgets = &mut style.visuals.widgets;
		for state in [
			&mut widgets.noninteractive,
			&mut widgets.inactive,
			&mut widgets.hovered,
			&mut widgets.active,
			&mut widgets.open,
		] {
			state.corner_radius = egui::CornerRadius::ZERO;
		}
	});
}

pub fn bar_frame(ui: &egui::Ui) -> egui::Frame {
	let visuals = ui.visuals();
	let fill = if visuals.dark_mode {
		egui::Color32::from_gray(36)
	} else {
		visuals.panel_fill
	};
	egui::Frame::side_top_panel(ui.style()).fill(fill)
}
