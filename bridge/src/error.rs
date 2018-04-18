#![allow(unknown_lints)]

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

// `CallResult` is a future whose associated type `Error` is a `web3::Error`

// `Timeout<F>` is a future whose associated type `Error` is `From<TimeoutError<F>>`

// so for `TimeoutError<CallResult<T, F>` the `Error` of CallResult (web::Error)
// must implement From<TimeoutError.
// that's pretty ridiculous
// so you need to wrap it

// the timeout has the same error type as the wrapped future.
// that's why the error type of the wrapped future must impl from TimeoutError

// you cant implement from TimeoutError for web3::Error

impl<F> From<TimeoutError<F>> for Error {
    fn from(err: TimeoutError<F>) -> Self {
        match err {
            TimeoutError::Timer(_, timer_error) => timer_error.into(),
            TimeoutError::TimedOut(_) => ErrorKind::TimedOut.into()
        }
    }
}

impl From<web3::Error> for Error {
    fn from(err: web3::Error) -> Self {
        ErrorKind::Web3(err).into()
    }
}

// /// captures a string describing the context
// pub fn annotate<T: AsRef<String>> (err: Error, context_description: T) -> {
//     format!("error occured while `{}`: {?}", context_description, err);
// }
