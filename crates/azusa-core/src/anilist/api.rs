//! `AniList` GraphQL API client.

use cynic::QueryBuilder;

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
}

#[derive(cynic::QueryFragment)]
#[cynic(graphql_type = "Media")]
pub struct Media {
	pub id: i32,
	pub id_mal: Option<i32>,
	pub title: Option<MediaTitle>,
	pub episodes: Option<i32>,
	pub season: Option<MediaSeason>,
	pub season_year: Option<i32>,
	pub cover_image: Option<MediaCoverImage>,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
pub enum MediaListStatus {
	Current,
	Completed,
	Paused,
	Dropped,
	Planning,
	Repeating,
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

/// A logged-in `AniList` session: the token plus the user it belongs to.
pub struct Session {
	pub token: AccessToken,
	pub viewer: Viewer,
}

impl Session {
	/// Validates the token by fetching the user it belongs to.
	///
	/// # Errors
	/// [`Error::Http`] if the request fails or the token is rejected.
	/// [`Error::GraphQl`] or [`Error::MissingData`] if the response has no viewer.
	pub fn start(token: AccessToken) -> Result<Self, Error> {
		let viewer = fetch_viewer(&token)?;
		Ok(Self { token, viewer })
	}

	/// Fetches the viewer's anime lists, one group per status or custom list.
	///
	/// # Errors
	/// [`Error::Http`] if the request fails or the token is rejected.
	/// [`Error::GraphQl`] or [`Error::MissingData`] if the response has no lists.
	pub fn anime_lists(&self) -> Result<Vec<MediaListGroup>, Error> {
		fetch_anime_list(self)
	}
}

fn fetch_viewer(token: &AccessToken) -> Result<Viewer, Error> {
	let response: cynic::GraphQlResponse<ViewerQuery> = ureq::post(API_URL)
		.header("Authorization", format!("Bearer {}", token.0))
		.header("Accept", "application/json")
		.send_json(ViewerQuery::build(()))?
		.body_mut()
		.read_json()?;

	if let Some(errors) = response.errors {
		return Err(Error::GraphQl(errors));
	}
	response
		.data
		.and_then(|data| data.viewer)
		.ok_or(Error::MissingData)
}

fn fetch_anime_list(session: &Session) -> Result<Vec<MediaListGroup>, Error> {
	let response: cynic::GraphQlResponse<MediaListCollectionQuery> = ureq::post(API_URL)
		.header("Authorization", format!("Bearer {}", session.token.0))
		.header("Accept", "application/json")
		.send_json(MediaListCollectionQuery::build(
			MediaListCollectionArguments {
				user_id: session.viewer.id,
			},
		))?
		.body_mut()
		.read_json()?;

	if let Some(errors) = response.errors {
		return Err(Error::GraphQl(errors));
	}
	response
		.data
		.and_then(|data| data.media_list_collection)
		.map(|collection| collection.lists)
		.ok_or(Error::MissingData)
}
