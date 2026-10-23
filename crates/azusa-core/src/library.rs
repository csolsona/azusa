use anitomy_ng::ElementKind;

#[derive(PartialEq, Debug)]
pub struct ParsedFile {
	pub title: String,
	pub episode: u32,
}

#[must_use]
pub fn parse_filename(name: &str) -> Option<ParsedFile> {
	let parsed = anitomy_ng::parse(name, anitomy_ng::Options::default());

	let value_of = |kind| {
		parsed
			.iter()
			.find(|element| element.kind == kind)
			.map(|element| element.value.clone())
	};

	let title = value_of(ElementKind::Title)?;
	let episode = value_of(ElementKind::Episode)?.parse::<u32>().ok()?;

	Some(ParsedFile { title, episode })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_single_episodes() {
		let cases = [
			(
				"[Erai-raws] Koori no Jouheki - 07 [1080p NF WEBRip HEVC AAC][MultiSub][F9D85957].mkv",
				"Koori no Jouheki",
				7,
			),
			(
				"[Erai-raws] Class de 2-banme ni Kawaii Onnanoko to Tomodachi ni Natta - 09 [1080p CR WEBRip HEVC AAC][MultiSub][38FC8548].mkv",
				"Class de 2-banme ni Kawaii Onnanoko to Tomodachi ni Natta",
				9,
			),
			(
				"[Erai-raws] Kamiina Botan - 04 [1080p CR WEBRip HEVC AAC][MultiSub][35D64441].mkv",
				"Kamiina Botan",
				4,
			),
			(
				"[Erai-raws] slime300 S2 - 12 [1080p CR WEBRip HEVC EAC3][MultiSub][3C468894].mkv",
				"slime300",
				12,
			),
			(
				"[Erai-raws] Koisuru Asteroid - 12 END [1080p][Multiple Subtitle].mkv",
				"Koisuru Asteroid",
				12,
			),
			(
				"[SubsPlease] Kimi ga Shinu made Koi wo Shitai - 05v2 (1080p) [27E5ACC1].mkv",
				"Kimi ga Shinu made Koi wo Shitai",
				5,
			),
		];

		for (name, title, episode) in cases {
			assert_eq!(
				parse_filename(name),
				Some(ParsedFile {
					title: title.into(),
					episode
				}),
				"case: {name}"
			);
		}
	}

	#[test]
	fn rejects_non_episode_files() {
		let cases = [
			"notes.txt",
			"[Erai-raws] Koisuru Asteroid - 06.5 [1080p][Multiple Subtitle].mkv",
		];

		for name in cases {
			assert_eq!(parse_filename(name), None, "case: {name}");
		}
	}
}
