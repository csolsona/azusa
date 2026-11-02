use std::{
	io,
	path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::library::{ParsedFile, parse_filename};

pub struct LocalEpisode {
	pub path: PathBuf,
	pub parsed: ParsedFile,
}

const VIDEO_EXTENSIONS: [&str; 4] = ["mkv", "mp4", "avi", "webm"];

/// Recursively collects the parseable video files under `root`.
///
/// # Errors
/// Returns an error if `root` itself can't be read.
/// Unreadable entries below it are skipped with a warning.
pub fn scan(root: &Path) -> io::Result<Vec<LocalEpisode>> {
	let mut episodes = Vec::new();

	for entry in WalkDir::new(root) {
		let entry = match entry {
			Ok(entry) => entry,
			Err(err) if err.depth() == 0 => return Err(err.into()),
			Err(err) => {
				log::warn!("skipping unreadable entry: {err}");
				continue;
			}
		};

		if !entry.file_type().is_file() {
			continue;
		}

		if !is_video(entry.path()) {
			continue;
		}

		let Some(name) = entry.file_name().to_str() else {
			continue;
		};

		let Some(parsed) = parse_filename(name) else {
			continue;
		};

		episodes.push(LocalEpisode {
			path: entry.into_path(),
			parsed,
		});
	}

	Ok(episodes)
}

pub(super) fn is_video(path: &Path) -> bool {
	path.extension()
		.and_then(|extension| extension.to_str())
		.is_some_and(|extension| {
			VIDEO_EXTENSIONS
				.iter()
				.any(|video| video.eq_ignore_ascii_case(extension))
		})
}

#[cfg(test)]
mod tests {
	use std::fs;

	use tempfile::tempdir;

	use super::*;

	#[test]
	fn finds_episodes() {
		let dir = tempdir().expect("tempdir should be created");

		let batch = dir
			.path()
			.join("[SubsPlease] Kimi ga Shinu made Koi wo Shitai (01-13) (1080p) [Batch]");

		fs::create_dir(&batch).expect("batch directory should be created");

		let files = [
			dir.path().join(
				"[Erai-raws] Koori no Jouheki - 07 [1080p NF WEBRip HEVC AAC][MultiSub][F9D85957].mkv",
			),
			dir.path().join(
				"[Erai-raws] Kamiina Botan - 04 [1080p CR WEBRip HEVC AAC][MultiSub][35D64441].mkv",
			),
			dir.path().join("notes.txt"),
			batch.join("[SubsPlease] Kimi ga Shinu made Koi wo Shitai - 01 (1080p) [0EA1E149].mkv"),
			batch.join("[SubsPlease] Kimi ga Shinu made Koi wo Shitai - 02 (1080p) [7ADD4E30].mkv"),
			batch.join(
				"[SubsPlease] Kimi ga Shinu made Koi wo Shitai - 03 (1080p) [0743A00F].mkv.part",
			),
			batch.join("[SubsPlease] Kimi ga Shinu made Koi wo Shitai - 04 (1080p) [F06DFED6].ass"),
			batch.join(
				"[SubsPlease] Kimi ga Shinu made Koi wo Shitai - 10v2 (1080p) [7E236C7D].mkv",
			),
			batch.join("README.txt"),
		];

		for file in files {
			fs::File::create(file).expect("test file should be created");
		}

		let mut episodes = scan(dir.path())
			.expect("directory should be scanned")
			.into_iter()
			.map(|local_episode| (local_episode.parsed.title, local_episode.parsed.episode))
			.collect::<Vec<_>>();
		episodes.sort();

		assert_eq!(
			episodes,
			vec![
				("Kamiina Botan".to_owned(), 4),
				("Kimi ga Shinu made Koi wo Shitai".to_owned(), 1),
				("Kimi ga Shinu made Koi wo Shitai".to_owned(), 2),
				("Kimi ga Shinu made Koi wo Shitai".to_owned(), 10),
				("Koori no Jouheki".to_owned(), 7),
			]
		);
	}

	#[test]
	fn fails_on_missing_root() {
		let dir = tempdir().expect("tempdir should be created");

		assert!(scan(&dir.path().join("missing")).is_err());
	}
}
