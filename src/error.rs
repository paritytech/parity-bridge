use std::{io, fmt};
use {web3, toml, docopt};

#[derive(Debug)]
pub enum Error {
	Args(docopt::Error),
	Setup(SetupError),
	Database(DatabaseError),
	App(AppError),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::Args(ref err) => fmt::Display::fmt(err, f),
			Error::Setup(ref _err) => f.write_str("Cannot load config file."),
			Error::Database(ref _err) => f.write_str("Cannot load or save database."),
			Error::App(ref err) => write!(f, "Runtime app err: {:?}", err),
		}
	}
}

impl From<docopt::Error> for Error {
	fn from(err: docopt::Error) -> Self {
		Error::Args(err)
	}
}

impl From<SetupError> for Error {
	fn from(err: SetupError) -> Self {
		Error::Setup(err)
	}
}

impl From<DatabaseError> for Error {
	fn from(err: DatabaseError) -> Self {
		Error::Database(err)
	}
}

impl From<AppError> for Error {
	fn from(err: AppError) -> Self {
		Error::App(err)
	}
}

#[derive(Debug)]
pub enum SetupError {
	Io(io::Error),
	Format(toml::de::Error),
}

impl From<io::Error> for SetupError {
	fn from(err: io::Error) -> Self {
		SetupError::Io(err)
	}
}

impl From<toml::de::Error> for SetupError {
	fn from(err: toml::de::Error) -> Self {
		SetupError::Format(err)
	}
}

#[derive(Debug)]
pub enum DatabaseError {
	Io(io::Error),
	Format(toml::de::Error),
}

impl From<io::Error> for DatabaseError {
	fn from(err: io::Error) -> Self {
		DatabaseError::Io(err)
	}
}

impl From<toml::de::Error> for DatabaseError {
	fn from(err: toml::de::Error) -> Self {
		DatabaseError::Format(err)
	}
}

#[derive(Debug)]
pub enum AppError {
	Io(io::Error),
	Web3(web3::Error),
}

impl From<io::Error> for AppError {
	fn from(err: io::Error) -> Self {
		AppError::Io(err)
	}
}

impl From<web3::Error> for AppError {
	fn from(err: web3::Error) -> Self {
		AppError::Web3(err)
	}
}
