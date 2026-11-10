//! `AniList` GraphQL API client.

use cynic::{MutationBuilder, QueryBuilder};

use super::schema;
use crate::anilist::{AccessToken, Error};

const API_URL: &str = "https://graphql.anilist.co";

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "Query")]
struct ViewerQuery {
	#[cynic(rename = "Viewer")]
	viewer: Option<Viewer>,
}

/// The authenticated `AniList` user.
#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "User")]
pub struct Viewer {
	pub id: i32,
	pub name: String,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "Query", variables = "MediaListCollectionArguments")]
struct MediaListCollectionQuery {
	#[arguments(userId: $user_id, type: ANIME)]
	#[cynic(rename = "MediaListCollection")]
	media_list_collection: Option<MediaListCollection>,
}

#[derive(cynic::QueryVariables)]
struct MediaListCollectionArguments {
	user_id: i32,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "MediaListCollection")]
struct MediaListCollection {
	#[cynic(flatten)]
	lists: Vec<MediaListGroup>,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "MediaListGroup")]
pub struct MediaListGroup {
	#[cynic(flatten)]
	pub entries: Vec<MediaList>,
	pub is_custom_list: Option<bool>,
	pub status: Option<MediaListStatus>,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "MediaList")]
pub struct MediaList {
	pub id: i32,
	pub media: Option<Media>,
	pub score: Option<f64>,
	pub progress: Option<i32>,
	pub private: Option<bool>,
	pub status: Option<MediaListStatus>,
	pub updated_at: Option<i32>,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "Media")]
pub struct Media {
	pub id: i32,
	pub id_mal: Option<i32>,
	pub title: Option<MediaTitle>,
	pub episodes: Option<i32>,
	pub duration: Option<i32>,
	pub format: Option<MediaFormat>,
	pub season: Option<MediaSeason>,
	pub season_year: Option<i32>,
	pub cover_image: Option<MediaCoverImage>,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(non_exhaustive)]
pub enum MediaFormat {
	Tv,
	TvShort,
	Movie,
	Special,
	Ova,
	Ona,
	Music,
	#[cynic(fallback)]
	Other,
}

#[derive(cynic::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaListStatus {
	Current,
	Completed,
	Paused,
	Dropped,
	Planning,
	Repeating,
}

impl MediaListStatus {
	pub const ALL: [Self; 6] = [
		Self::Current,
		Self::Completed,
		Self::Paused,
		Self::Dropped,
		Self::Planning,
		Self::Repeating,
	];
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "MediaCoverImage")]
pub struct MediaCoverImage {
	pub extra_large: Option<String>,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "MediaTitle")]
pub struct MediaTitle {
	pub english: Option<String>,
	pub native: Option<String>,
	pub romaji: Option<String>,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
pub enum MediaSeason {
	Winter,
	Spring,
	Summer,
	Fall,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "Mutation", variables = "SaveProgressArguments")]
struct SaveProgressMutation {
	#[arguments(id: $id, progress: $progress)]
	#[cynic(rename = "SaveMediaListEntry")]
	save_media_list_entry: Option<SavedProgress>,
}

#[derive(cynic::QueryVariables)]
struct SaveProgressArguments {
	id: i32,
	progress: i32,
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "MediaList")]
struct SavedProgress {
	progress: Option<i32>,
}

/// A logged-in `AniList` session: the token plus the user it belongs to.
pub struct Session {
	pub token: AccessToken,
	pub viewer: Viewer,
}

impl Session {
	/// Validates the token by fetching the user it belongs to.
	///
	/// # Errors
	/// [`Error::Http`] if the request fails.
	/// [`Error::GraphQl`] if `AniList` rejects it, for example an expired token.
	/// [`Error::MissingData`] if the response has no viewer.
	pub fn start(token: AccessToken) -> Result<Self, Error> {
		let viewer = send(&token, ViewerQuery::build(()))?
			.viewer
			.ok_or(Error::MissingData)?;

		Ok(Self { token, viewer })
	}

	/// Fetches the viewer's anime lists, one group per status or custom list.
	///
	/// # Errors
	/// [`Error::Http`] if the request fails.
	/// [`Error::GraphQl`] if `AniList` rejects it, for example an expired token.
	/// [`Error::MissingData`] if the response has no lists.
	pub fn anime_lists(&self) -> Result<Vec<MediaListGroup>, Error> {
		let operation = MediaListCollectionQuery::build(MediaListCollectionArguments {
			user_id: self.viewer.id,
		});

		send(&self.token, operation)?
			.media_list_collection
			.map(|collection| collection.lists)
			.ok_or(Error::MissingData)
	}

	/// Sets the episode progress of a list entry and returns the progress `AniList` saved.
	///
	/// # Errors
	/// [`Error::Http`] if the request fails.
	/// [`Error::GraphQl`] if `AniList` rejects it, for example an expired token.
	/// [`Error::MissingData`] if the response has no entry.
	pub fn update_progress(&self, entry_id: i32, progress: i32) -> Result<i32, Error> {
		let operation = SaveProgressMutation::build(SaveProgressArguments {
			id: entry_id,
			progress,
		});

		send(&self.token, operation)?
			.save_media_list_entry
			.and_then(|entry| entry.progress)
			.ok_or(Error::MissingData)
	}
}

fn send<T, V>(token: &AccessToken, operation: cynic::Operation<T, V>) -> Result<T, Error>
where
	T: cynic::serde::de::DeserializeOwned,
	V: cynic::serde::Serialize,
{
	let response: cynic::GraphQlResponse<T> = ureq::post(API_URL)
		.config()
		.http_status_as_error(false)
		.build()
		.header("Authorization", format!("Bearer {}", token.0))
		.header("Accept", "application/json")
		.send_json(operation)?
		.body_mut()
		.read_json()?;

	if let Some(errors) = response.errors {
		return Err(Error::GraphQl(errors));
	}

	response.data.ok_or(Error::MissingData)
}
