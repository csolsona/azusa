use azusa_core::anilist::MediaListStatus;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, selected: &mut MediaListStatus) {
	let row = ui.horizontal(|ui| {
		ui.spacing_mut().item_spacing.x = 0.0;
		let mut selected_rect = None;

		for status in MediaListStatus::ALL {
			let is_selected = *selected == status;
			let response = status_tab(ui, status_label(status), is_selected);

			if is_selected {
				selected_rect = Some(response.rect);
			}
			if response.clicked() {
				*selected = status;
			}
		}

		selected_rect
	});

	if let Some(tab) = row.inner {
		let border = ui.visuals().widgets.noninteractive.bg_stroke;
		let area = ui.clip_rect();
		let painter = ui.painter();

		// The gap under the selected tab makes it look connected to the table.
		if tab.left() > area.left() {
			painter.hline(area.left()..=tab.left(), tab.bottom(), border);
		}
		painter.hline(tab.right()..=area.right(), tab.bottom(), border);
	}
}

fn status_tab(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
	let response = ui
		.scope(|ui| {
			let visuals = ui.visuals_mut();
			let (rest, hover) = if selected {
				(visuals.panel_fill, visuals.panel_fill)
			} else {
				(
					visuals.faint_bg_color,
					visuals.selection.bg_fill.gamma_multiply(0.4),
				)
			};

			let widgets = &mut visuals.widgets;
			widgets.inactive.weak_bg_fill = rest;
			for state in [&mut widgets.hovered, &mut widgets.active] {
				state.weak_bg_fill = hover;
				state.bg_stroke = egui::Stroke::NONE;
			}

			ui.add(egui::Button::new(label).min_size(egui::vec2(0.0, 22.0)))
		})
		.inner;

	if selected {
		let rect = response.rect;
		let border = ui.visuals().widgets.noninteractive.bg_stroke;
		let emphasis = egui::Stroke::new(2.0, ui.visuals().strong_text_color());
		let painter = ui.painter();

		// At the panel edge, the sidebar's border already acts as the tab's left border.
		if rect.left() > ui.clip_rect().left() {
			painter.vline(rect.left(), rect.y_range(), border);
		}
		painter.vline(rect.right(), rect.y_range(), border);
		painter.hline(rect.x_range(), rect.top(), emphasis);
	}

	response
}

fn status_label(status: MediaListStatus) -> &'static str {
	match status {
		MediaListStatus::Current => "Watching",
		MediaListStatus::Completed => "Completed",
		MediaListStatus::Paused => "Paused",
		MediaListStatus::Dropped => "Dropped",
		MediaListStatus::Planning => "Planning",
		MediaListStatus::Repeating => "Rewatching",
	}
}
