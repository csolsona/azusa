use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Config {
	pub library: LibraryConfig,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryConfig {
	pub folders: Vec<PathBuf>,
}
