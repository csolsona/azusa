const VIDEO_PLAYERS: [&str; 2] = ["mpv", "haruna"];

/// Returns the file name of the video open in a video player, if any.
#[cfg(target_os = "linux")]
#[must_use]
pub fn now_playing() -> Option<String> {
	use crate::library::scan::is_video;
	use std::fs;

	for entry in fs::read_dir("/proc").ok()?.flatten() {
		let process = entry.path();

		let Ok(name) = fs::read_to_string(process.join("comm")) else {
			continue;
		};

		if !VIDEO_PLAYERS.contains(&name.trim()) {
			continue;
		}

		let Ok(files) = fs::read_dir(process.join("fd")) else {
			continue;
		};

		for file in files.flatten() {
			let Ok(target) = fs::read_link(file.path()) else {
				continue;
			};

			if is_video(&target) {
				return target.file_name()?.to_str().map(String::from);
			}
		}
	}

	None
}
