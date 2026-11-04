use std::collections::HashMap;

use azusa_core::{
	anilist::{MediaListGroup, MediaListStatus},
	library::LocalEpisode,
};
use eframe::egui;

use crate::sync::ProgressChange;

use super::{status_tabs, table};

pub fn show(
	ui: &mut egui::Ui,
	lists: &[MediaListGroup],
	selected: &mut MediaListStatus,
	matched: Option<&HashMap<i32, Vec<LocalEpisode>>>,
) -> Option<ProgressChange> {
	status_tabs::show(ui, selected);

	let Some(group) = lists.iter().find(|group| group.status == Some(*selected)) else {
		ui.label("No lists");
		return None;
	};

	ui.style_mut().interaction.selectable_labels = false;
	table::show(ui, group, matched)
}
