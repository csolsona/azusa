const CLIENT_ID: &str = "41998";

/// Builds the `AniList` OAuth implicit-grant authorization URL.
#[must_use]
pub fn authorize_url() -> String {
	format!("https://anilist.co/api/v2/oauth/authorize?client_id={CLIENT_ID}&response_type=token")
}

pub struct AccessToken(String);

// /token?access_token=abc&token_type=Bearer&expires_in=31536000
fn token_from_path(path: &str) -> Option<AccessToken> {
	path.strip_prefix("/token?")?
		.split('&')
		.find_map(|pair| pair.strip_prefix("access_token="))
		.filter(|token| !token.is_empty())
		.map(|token| AccessToken(token.to_owned()))
}

/// Local HTTP server listening on the `AniList` redirect URL.
pub struct LoginServer(tiny_http::Server);

impl LoginServer {
	/// Binds the local server so the browser redirect has somewhere to land.
	///
	/// # Errors
	/// [`AuthError::Bind`] if port 27279 is taken.
	pub fn bind() -> Result<Self, AuthError> {
		// Must match the redirect URL registered in the AniList client settings.
		let server = tiny_http::Server::http("127.0.0.1:27279").map_err(AuthError::Bind)?;
		Ok(Self(server))
	}

	/// Blocks until the browser delivers the `AniList` token.
	///
	/// # Errors
	/// [`AuthError::Timeout`] after 5 minutes without login, [`AuthError::Io`] if the HTTP exchange fails.
	///
	/// # Panics
	/// Never in practice: the only `expect` is on a static, valid header.
	pub fn wait_for_token(self) -> Result<AccessToken, AuthError> {
		loop {
			let Some(request) = self.0.recv_timeout(std::time::Duration::from_secs(300))? else {
				return Err(AuthError::Timeout);
			};

			if let Some(token) = token_from_path(request.url()) {
				// The token is already received; failing to show the confirmation page shouldn't fail the login.
				let _ = request.respond(tiny_http::Response::from_string(
					"Login complete, you can close this tab.",
				));
				return Ok(token);
			}

			if request.url() == "/callback" {
				request.respond(
					tiny_http::Response::from_string(CALLBACK_HTML).with_header(
						tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8")
							.expect("static header is valid"),
					),
				)?;
			} else {
				request
					.respond(tiny_http::Response::from_string("Not found").with_status_code(404))?;
			}
		}
	}
}

// The browser never sends the URL fragment to the server, so we bounce it into a query string.
const CALLBACK_HTML: &str =
	"<script>location.replace('/token?' + location.hash.slice(1));</script>";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
	#[error("could not start local server: {0}")]
	Bind(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("login timed out")]
	Timeout,
	#[error("local server I/O failed: {0}")]
	Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_token_from_callback() {
		let token =
			token_from_path("/token?access_token=abc&token_type=Bearer&expires_in=31536000")
				.expect("token should be parsed");
		assert_eq!(token.0, "abc");
	}

	#[test]
	fn parses_token_in_any_position() {
		let token = token_from_path("/token?token_type=Bearer&access_token=abc")
			.expect("token should be parsed");
		assert_eq!(token.0, "abc");
	}

	#[test]
	fn rejects_missing_token() {
		assert!(token_from_path("/token?token_type=Bearer").is_none());
	}

	#[test]
	fn rejects_empty_token() {
		assert!(token_from_path("/token?access_token=&token_type=Bearer").is_none());
	}

	#[test]
	fn rejects_other_paths() {
		assert!(token_from_path("/callback").is_none());
		assert!(token_from_path("/favicon.ico").is_none());
	}

	#[test]
	#[ignore = "manual: needs a browser login"]
	fn manual_login() {
		let token = LoginServer::bind()
			.expect("port should be free")
			.wait_for_token()
			.expect("login should succeed");
		println!("{}", token.0);
	}
}
