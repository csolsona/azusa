#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("could not start local server: {0}")]
	Bind(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("login timed out")]
	Timeout,
	#[error("local server I/O failed: {0}")]
	Io(#[from] std::io::Error),
	#[error("AniList request failed: {0}")]
	Http(#[from] ureq::Error),
	#[error("AniList returned errors: {0:?}")]
	GraphQl(Vec<cynic::GraphQlError>),
	#[error("AniList response had no data")]
	MissingData,
	#[error("keyring error: {0:?}")]
	Keyring(#[from] keyring::Error),
}
