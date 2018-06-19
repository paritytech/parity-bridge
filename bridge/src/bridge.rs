use futures::{Async, Poll, Stream};
use web3::Transport;

use database::State;
use error::{self, ResultExt};
use log_stream::LogStream;
use main_contract::MainContract;
use main_to_side_sign;
use relay_stream::RelayStream;
use side_contract::SideContract;
use side_to_main_sign;
use side_to_main_signatures;

/// combines relays streams with the database.
/// (relay streams have no knowledge of the database.)
/// wraps the relay streams.
/// if polled polls all relay streams which causes them fetch
/// all pending relays and relay them
/// updates the database with results returned from relay streams.
pub struct Bridge<T: Transport> {
    main_to_side_sign: RelayStream<LogStream<T>, main_to_side_sign::LogToMainToSideSign<T>>,
    side_to_main_sign: RelayStream<LogStream<T>, side_to_main_sign::LogToSideToMainSign<T>>,
    side_to_main_signatures:
        RelayStream<LogStream<T>, side_to_main_signatures::LogToSideToMainSignatures<T>>,
    state: State,
}

impl<T: Transport> Bridge<T> {
    pub fn new(
        initial_state: State,
        main_contract: MainContract<T>,
        side_contract: SideContract<T>,
    ) -> Self {
        let main_to_side_sign = RelayStream::new(
            main_contract.main_to_side_log_stream(initial_state.last_main_to_side_sign_at_block),
            main_to_side_sign::LogToMainToSideSign {
                side: side_contract.clone(),
            },
        );

        let side_to_main_sign = RelayStream::new(
            side_contract
                .side_to_main_sign_log_stream(initial_state.last_side_to_main_sign_at_block),
            side_to_main_sign::LogToSideToMainSign {
                side: side_contract.clone(),
            },
        );

        let side_to_main_signatures = RelayStream::new(
            side_contract.side_to_main_signatures_log_stream(
                initial_state.last_side_to_main_signatures_at_block,
            ),
            side_to_main_signatures::LogToSideToMainSignatures {
                main: main_contract.clone(),
                side: side_contract.clone(),
            },
        );

        Self {
            main_to_side_sign,
            side_to_main_sign,
            side_to_main_signatures,
            state: initial_state,
        }
    }
}

impl<T: Transport> Stream for Bridge<T> {
    type Item = State;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let maybe_main_to_side_sign = try_maybe_stream!(
                self.main_to_side_sign
                    .poll()
                    .chain_err(|| "Bridge: polling main to side sign failed")
            );
            let maybe_side_to_main_sign = try_maybe_stream!(
                self.side_to_main_sign
                    .poll()
                    .chain_err(|| "Bridge: polling side to main sign failed")
            );
            let maybe_side_to_main_signatures = try_maybe_stream!(
                self.side_to_main_signatures
                    .poll()
                    .chain_err(|| "Bridge: polling side to main signatures failed")
            );

            let mut has_state_changed = false;

            if let Some(main_to_side_sign) = maybe_main_to_side_sign {
                info!(
                    "last block checked for main to side sign is now {}",
                    main_to_side_sign
                );
                self.state.last_main_to_side_sign_at_block = main_to_side_sign;
                has_state_changed = true;
            }
            if let Some(side_to_main_sign) = maybe_side_to_main_sign {
                info!(
                    "last block checked for side to main sign is now {}",
                    side_to_main_sign
                );
                self.state.last_side_to_main_sign_at_block = side_to_main_sign;
                has_state_changed = true;
            }
            if let Some(side_to_main_signatures) = maybe_side_to_main_signatures {
                info!(
                    "last block checked for side to main signatures is now {}",
                    side_to_main_signatures
                );
                self.state.last_side_to_main_signatures_at_block = side_to_main_signatures;
                has_state_changed = true;
            }

            if has_state_changed {
                return Ok(Async::Ready(Some(self.state.clone())));
            } else {
                return Ok(Async::NotReady);
            }
        }
    }
}
