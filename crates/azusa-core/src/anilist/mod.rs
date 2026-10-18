mod api;
mod auth;
mod error;

pub use api::{
	Media, MediaCoverImage, MediaFormat, MediaList, MediaListGroup, MediaListStatus, MediaSeason,
	MediaTitle, Session, Viewer,
};
pub use auth::{AccessToken, LoginServer, authorize_url};
pub use error::Error;

#[cynic::schema("anilist")]
mod schema {}
