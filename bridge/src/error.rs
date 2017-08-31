#![allow(unknown_lints)]

use std::io;
use api::ApiCall;
use tokio_timer::{TimerError, TimeoutError};
use {web3, toml, ethabi};

error_chain! {
	types {
		Error, ErrorKind, ResultExt, Result;
	}

	foreign_links {
		Io(io::Error);
		Toml(toml::de::Error);
		Ethabi(ethabi::Error);
		Timer(TimerError);
	}

	errors {
		// api timeout
		Timeout(request: &'static str) {
			description("Request timeout"),
			display("Request {} timed out", request),
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

impl<T, F> From<TimeoutError<ApiCall<T, F>>> for Error {
	fn from(err: TimeoutError<ApiCall<T, F>>) -> Self {
		match err {
			TimeoutError::Timer(call, _) | TimeoutError::TimedOut(call) => {
				ErrorKind::Timeout(call.message()).into()
			}
		}
	}
}
