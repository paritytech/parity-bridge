use std::{io, fmt};
use {web3, toml, docopt};

error_chain! {
	types {
		Error, ErrorKind, ResultExt, Result;
	}

	foreign_links {
		Io(io::Error);
		Toml(toml::de::Error);
		Docopt(docopt::Error);
	}

	errors {
		// workaround for lack of web3:Error Display and Error implementations
		Web3(err: web3::Error) {
			description("web3 error")
			display("{:?}", err)
		}
	}
}
