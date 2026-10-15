//! `AniList` GraphQL API client.

use serde::{Deserialize, Serialize};

use crate::anilist::{AccessToken, Error};

const API_URL: &str = "https://graphql.anilist.co";
const VIEWER_QUERY: &str = "{ Viewer { name } }";

/// The authenticated `AniList` user.
#[derive(Deserialize)]
pub struct Viewer {
	pub name: String,
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
	pub fn start(token: AccessToken) -> Result<Self, Error> {
		let viewer = fetch_viewer(&token)?;
		Ok(Self { token, viewer })
	}
}

/// Body of a GraphQL request.
#[derive(Serialize)]
struct GraphQlRequest {
	query: &'static str,
}

/// Response to [`VIEWER_QUERY`]: `{"data": {"Viewer": {...}}}`.
#[derive(Deserialize)]
struct ViewerResponse {
	data: ViewerData,
}

#[derive(Deserialize)]
struct ViewerData {
	#[serde(rename = "Viewer")]
	viewer: Viewer,
}

fn fetch_viewer(token: &AccessToken) -> Result<Viewer, Error> {
	let response: ViewerResponse = ureq::post(API_URL)
		.header("Authorization", format!("Bearer {}", token.0))
		.header("Accept", "application/json")
		.send_json(GraphQlRequest {
			query: VIEWER_QUERY,
		})?
		.body_mut()
		.read_json()?;
	Ok(response.data.viewer)
}
