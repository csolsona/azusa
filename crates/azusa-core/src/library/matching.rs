use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

use crate::{
	anilist::MediaListGroup,
	library::{LocalEpisode, parse_filename},
};

/// Groups downloaded episodes by the listed anime they match.
/// Unknown or ambiguous titles are dropped.
#[must_use]
pub fn match_library(
	lists: &[MediaListGroup],
	episodes: Vec<LocalEpisode>,
) -> HashMap<i32, Vec<LocalEpisode>> {
	let entries = anime_titles(lists);

	let index = title_index(&entries);

	match_episodes(&index, episodes)
}

#[derive(Debug, PartialEq)]
/// The listed anime and episode a video file belongs to.
pub struct MatchedFile {
	pub media_id: i32,
	pub episode: u32,
}

/// Identifies a video file by its name.
/// Returns `None` for unparseable names and unknown or ambiguous titles.
#[must_use]
pub fn match_file(lists: &[MediaListGroup], file_name: &str) -> Option<MatchedFile> {
	let parsed = parse_filename(file_name)?;

	let entries = anime_titles(lists);
	let index = title_index(&entries);
	let media_id = find_anime(&index, &parsed.title)?;

	Some(MatchedFile {
		media_id,
		episode: parsed.episode,
	})
}

/// An anime from the user's lists with the titles it can be matched by.
struct AnimeTitles<'a> {
	id: i32,
	titles: Vec<&'a str>,
}

/// Collects each listed anime's id with its romaji and English titles.
fn anime_titles(lists: &[MediaListGroup]) -> Vec<AnimeTitles<'_>> {
	let mut entries = Vec::new();

	for list in lists {
		for entry in &list.entries {
			let Some(media) = &entry.media else { continue };
			let Some(media_title) = &media.title else {
				continue;
			};

			let titles = [
				media_title.romaji.as_deref(),
				media_title.english.as_deref(),
			]
			.into_iter()
			.flatten()
			.collect();

			entries.push(AnimeTitles {
				id: media.id,
				titles,
			});
		}
	}

	entries
}

#[derive(Debug, PartialEq, Eq)]
enum TitleMatch {
	Unique(i32),
	Ambiguous(Vec<i32>),
}

/// Maps each normalized title to the anime that use it.
fn title_index(entries: &[AnimeTitles]) -> HashMap<String, TitleMatch> {
	let mut index = HashMap::new();

	for AnimeTitles { id, titles } in entries {
		for title in titles {
			let normalized = normalize_title(title);

			if normalized.is_empty() {
				continue;
			}

			index
				.entry(normalized)
				.and_modify(|existing| match existing {
					// e.g. "K-ON!" and "K-ON!!" both normalize to "kon".
					TitleMatch::Unique(first) if *first != *id => {
						*existing = TitleMatch::Ambiguous(vec![*first, *id]);
					}
					TitleMatch::Ambiguous(ids) if !ids.contains(id) => ids.push(*id),
					_ => {}
				})
				.or_insert(TitleMatch::Unique(*id));
		}
	}

	index
}

/// Returns the anime a title unambiguously belongs to.
fn find_anime(index: &HashMap<String, TitleMatch>, title: &str) -> Option<i32> {
	match index.get(&normalize_title(title)) {
		Some(TitleMatch::Unique(id)) => Some(*id),
		_ => None,
	}
}

/// Groups episodes by the anime they unambiguously match, dropping the rest.
fn match_episodes(
	index: &HashMap<String, TitleMatch>,
	episodes: Vec<LocalEpisode>,
) -> HashMap<i32, Vec<LocalEpisode>> {
	let mut episodes_by_anime: HashMap<i32, Vec<LocalEpisode>> = HashMap::new();

	for episode in episodes {
		let Some(id) = find_anime(index, &episode.parsed.title) else {
			continue;
		};

		episodes_by_anime.entry(id).or_default().push(episode);
	}

	episodes_by_anime
}

/// Normalizes a title for comparison: lowercases, unifies macron romanization, strips accents and keeps only alphanumeric characters.
fn normalize_title(title: &str) -> String {
	let lowercase = title.to_lowercase();
	let romanized = lowercase
		.replace(['ō', 'ô'], "ou")
		.replace(['ū', 'û'], "uu");
	romanized.nfkd().filter(|c| c.is_alphanumeric()).collect()
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use crate::{
		anilist::{Media, MediaList, MediaTitle},
		library::ParsedFile,
	};

	use super::*;

	#[test]
	fn normalizes_titles() {
		let cases = [
			("Koori no Jouheki", "koorinojouheki"),
			("Ichijouma Mankitsu-gurashi!", "ichijoumamankitsugurashi"),
			(
				"Class de 2-banme ni Kawaii Onnanoko to Tomodachi ni Natta",
				"classde2banmenikawaiionnanokototomodachininatta",
			),
			(
				"Anime: super complex, and_su-per.fun~Title!!",
				"animesupercomplexandsuperfuntitle",
			),
			("Pokémon", "pokemon"),
			("Kōkaku Kidōtai", "koukakukidoutai"),
			("ＡＢＣ", "abc"),
			("氷の城壁", "氷の城壁"),
			("!!!", ""),
		];

		for (title, normalized) in cases {
			assert_eq!(normalize_title(title), normalized, "case: {title}");
		}
	}

	#[test]
	fn indexes_every_title() {
		let index = title_index(&[
			AnimeTitles {
				id: 186_497,
				titles: vec!["Koori no Jouheki", "The Ramparts of Ice"],
			},
			AnimeTitles {
				id: 187_869,
				titles: vec!["Kamiina Botan, Yoeru Sugata wa Yuri no Hana"],
			},
		]);

		assert_eq!(
			index.get("koorinojouheki"),
			Some(&TitleMatch::Unique(186_497))
		);
		assert_eq!(
			index.get("therampartsofice"),
			Some(&TitleMatch::Unique(186_497))
		);
		assert_eq!(
			index.get("kamiinabotanyoerusugatawayurinohana"),
			Some(&TitleMatch::Unique(187_869))
		);
		assert_eq!(index.len(), 3);
	}

	#[test]
	fn same_title_twice_keeps_its_id() {
		// K-ON! has identical romaji and english titles.
		let index = title_index(&[AnimeTitles {
			id: 5680,
			titles: vec!["K-ON!", "K-ON!"],
		}]);

		assert_eq!(index.get("kon"), Some(&TitleMatch::Unique(5680)));
	}

	#[test]
	fn shared_title_is_ambiguous() {
		let index = title_index(&[
			AnimeTitles {
				id: 5680,
				titles: vec!["K-ON!"],
			},
			AnimeTitles {
				id: 7791,
				titles: vec!["K-ON!"],
			},
		]);

		assert_eq!(
			index.get("kon"),
			Some(&TitleMatch::Ambiguous(vec![5680, 7791]))
		);
	}

	#[test]
	fn ambiguity_collects_every_candidate_once() {
		let index = title_index(&[
			AnimeTitles {
				id: 1,
				titles: vec!["K-ON!"],
			},
			AnimeTitles {
				id: 2,
				titles: vec!["K-ON!!", "K-ON!!"],
			},
			AnimeTitles {
				id: 3,
				titles: vec!["K-ON!!!"],
			},
		]);

		assert_eq!(
			index.get("kon"),
			Some(&TitleMatch::Ambiguous(vec![1, 2, 3]))
		);
	}

	#[test]
	fn groups_matched_episodes() {
		let index = title_index(&[
			AnimeTitles {
				id: 186_497,
				titles: vec!["Koori no Jouheki"],
			},
			AnimeTitles {
				id: 5680,
				titles: vec!["K-ON!"],
			},
			AnimeTitles {
				id: 7791,
				titles: vec!["K-ON!"],
			},
		]);

		let episode = |title: &str, number: u32| LocalEpisode {
			path: PathBuf::new(),
			parsed: ParsedFile {
				title: title.to_string(),
				episode: number,
			},
		};
		let episodes = vec![
			episode("Koori no Jouheki", 1),
			episode("Koori no Jouheki", 2),
			episode("K-ON!", 1),
			episode("randomfile 8379234283019347971", 1),
		];

		let result = match_episodes(&index, episodes);

		assert_eq!(result.len(), 1);
		assert_eq!(result[&186_497].len(), 2);
	}

	fn list_with(id: i32, romaji: &str) -> MediaListGroup {
		let title = MediaTitle {
			romaji: Some(romaji.to_string()),
			english: None,
			native: None,
		};
		let media = Media {
			id,
			id_mal: None,
			title: Some(title),
			episodes: None,
			format: None,
			season: None,
			season_year: None,
			cover_image: None,
		};
		let entry = MediaList {
			id: 0,
			media: Some(media),
			score: None,
			progress: None,
			private: None,
			status: None,
			updated_at: None,
		};
		MediaListGroup {
			entries: vec![entry],
			is_custom_list: None,
			status: None,
		}
	}

	#[test]
	fn matches_file_to_listed_anime() {
		let lists = [list_with(186_497, "Koori no Jouheki")];
		let file = "[Erai-raws] Koori no Jouheki - 06 [1080p][3722B06F].mkv";

		assert_eq!(
			match_file(&lists, file),
			Some(MatchedFile {
				media_id: 186_497,
				episode: 6
			})
		);
		assert_eq!(match_file(&lists, "[Erai-raws] Other Anime - 06.mkv"), None);
	}
}
