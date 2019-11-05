// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.

//! error chain

use std::io;
use tokio_timer::{TimeoutError, TimerError};
use {ethabi, rustc_hex, toml, web3};

error_chain! {
	types {
		Error, ErrorKind, ResultExt, Result;
	}

	foreign_links {
		Io(io::Error);
		Toml(toml::de::Error);
		Ethabi(ethabi::Error);
		Timer(TimerError);
		Hex(rustc_hex::FromHexError);
	}

	errors {
		TimedOut {
			description("Request timed out"),
			display("Request timed out"),
		}
		// workaround for error_chain not allowing to check internal error kind
		// https://github.com/rust-lang-nursery/error-chain/issues/206
		MissingFile(filename: String) {
			description("File not found"),
			display("File {} not found", filename),
		}
		// workaround for lack of web3:Error Display and Error implementations
		Web3(err: web3::Error) {
			description("web3 error"),
			display("{:?}", err),
		}
	}
}

// tokio timer `Timeout<F>` can only wrap futures `F` whose assocaited `Error` type
// satisfies `From<TimeoutError<F>>`
//
// `web3::CallResult`'s associated error type `Error` which is `web3::Error`
// does not satisfy `From<TimeoutError<F>>`.
// thus we can't use `Timeout<web3::CallResult>`.
// we also can't implement `From<TimeoutError<F>` for `web3::Error` since
// we control neither of the types.
//
// instead we implement `TimeoutError<F>` for `Error` and `From<web3::Error>`
// for `Error` so we can convert `web3::Error` into `Error` and then use that
// with `Timeout`.

impl<F> From<TimeoutError<F>> for Error {
	fn from(err: TimeoutError<F>) -> Self {
		match err {
			TimeoutError::Timer(_, timer_error) => timer_error.into(),
			TimeoutError::TimedOut(_) => ErrorKind::TimedOut.into(),
		}
	}
}

impl From<web3::Error> for Error {
	fn from(err: web3::Error) -> Self {
		ErrorKind::Web3(err).into()
	}
}
