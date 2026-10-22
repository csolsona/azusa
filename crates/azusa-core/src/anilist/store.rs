use keyring::Entry;

use crate::anilist::{AccessToken, Error};

const SERVICE: &str = "azusa";
const USER: &str = "anilist";

pub fn save_token(token: &AccessToken) -> Result<(), Error> {
	entry()?.set_password(&token.0)?;
	Ok(())
}

pub fn load_token() -> Result<Option<AccessToken>, Error> {
	match entry()?.get_password() {
		Ok(token) => Ok(Some(AccessToken(token))),
		Err(keyring::Error::NoEntry) => Ok(None),
		Err(err) => Err(err.into()),
	}
}

pub fn delete_token() -> Result<(), Error> {
	match entry()?.delete_credential() {
		Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
		Err(err) => Err(err.into()),
	}
}

fn entry() -> keyring::Result<Entry> {
	Entry::new(SERVICE, USER)
}
