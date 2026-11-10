use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
	pub library: LibraryConfig,
	pub playback: PlaybackConfig,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryConfig {
	pub folders: Vec<PathBuf>,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackConfig {
	pub progress_update: ProgressUpdate,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ProgressUpdate {
	Manual,
	Auto(AutoUpdateConfig),
}

impl Default for ProgressUpdate {
	fn default() -> Self {
		Self::Auto(AutoUpdateConfig::default())
	}
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoUpdateConfig {
	pub required_percent: u8,
}

impl Default for AutoUpdateConfig {
	fn default() -> Self {
		Self {
			required_percent: 50,
		}
	}
}
