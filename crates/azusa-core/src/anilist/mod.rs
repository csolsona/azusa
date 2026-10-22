mod api;
mod auth;
mod error;
mod store;

pub use api::{
	Media, MediaCoverImage, MediaFormat, MediaList, MediaListGroup, MediaListStatus, MediaSeason,
	MediaTitle, Session, Viewer,
};
pub use auth::{AccessToken, LoginServer, authorize_url};
pub use error::Error;
pub use store::{delete_token, load_token, save_token};

#[cynic::schema("anilist")]
mod schema {}
