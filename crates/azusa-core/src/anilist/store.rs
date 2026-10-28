use keyring::Entry;

use crate::anilist::{AccessToken, Error};

const SERVICE: &str = "azusa";
const USER: &str = "anilist";

/// Stores the token in the system keyring.
///
/// # Errors
/// [`Error::Keyring`] if the keyring is unavailable or rejects the write.
pub fn save_token(token: &AccessToken) -> Result<(), Error> {
	entry()?.set_password(&token.0)?;
	Ok(())
}

/// Loads the token stored in the system keyring.
///
/// # Errors
/// [`Error::Keyring`] if the keyring is unavailable or the read fails.
pub fn load_token() -> Result<Option<AccessToken>, Error> {
	match entry()?.get_password() {
		Ok(token) => Ok(Some(AccessToken(token))),
		Err(keyring::Error::NoEntry) => Ok(None),
		Err(err) => Err(err.into()),
	}
}

/// Deletes the token stored in the system keyring.
///
/// # Errors
/// [`Error::Keyring`] if the keyring is unavailable or rejects the deletion.
pub fn delete_token() -> Result<(), Error> {
	match entry()?.delete_credential() {
		Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
		Err(err) => Err(err.into()),
	}
}

fn entry() -> keyring::Result<Entry> {
	Entry::new(SERVICE, USER)
}
