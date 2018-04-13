/// extraction of a pattern that occurred repeatedly in the codebase
///
/// where a "relay" is the detection of an event on chain A
/// followed by a transaction on chain B

use web3::types::Log;
use futures::{Future, Poll, Stream, Async};
use futures::future::{join_all, JoinAll};
use web3::Transport;
use log_stream::LogStream;
use error;

/// something that can create relay futures from logs.
/// called by `RelayStream` for every log.
pub trait RelayFactory {
    type Relay: Future<Error=error::Error>;

    fn log_to_relay(&self, log: Log) -> Self::Relay;
}

/// state of the state machine that is the relay stream
enum RelayStreamState<R: Future> {
    WaitForLogs,
    WaitForRelays {
        future: JoinAll<Vec<R>>,
        block: u64,
    },
}

/// a tokio `Stream` that when polled fetches all new logs from `logs`
/// executes a `ForeignBridge.deposit`
/// stream yields last block number on `home` for which all `HomeBrige.Deposit`
/// events have been handled this way.
pub struct RelayStream<T: Transport, F: RelayFactory> {
    logs: LogStream<T>,
    relay_factory: F,
    state: RelayStreamState<F::Relay>
}

impl<T: Transport + Clone, F: RelayFactory> RelayStream<T, F> {
    pub fn new(
        logs: LogStream<T>,
        relay_factory: F,
    ) -> Self {
        Self {
            logs,
            relay_factory,
            state: RelayStreamState::WaitForLogs,
        }
    }
}

impl<T: Transport + Clone, F: RelayFactory> Stream for RelayStream<T, F> {
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let (next_state, value_to_yield) = match self.state {
                RelayStreamState::WaitForLogs => {
                    let log_range = try_stream!(self.logs.poll());

                    // borrow checker...
                    let relay_factory = &self.relay_factory;

                    let relays = log_range.logs
                        .into_iter()
                        .map(|log| relay_factory.log_to_relay(log))
                        .collect::<Vec<_>>();

                    (RelayStreamState::WaitForRelays {
                        future: join_all(relays),
                        block: log_range.to,
                    }, None)
                }
                RelayStreamState::WaitForRelays { ref mut future, block } => {
                    try_ready!(future.poll());
                    (RelayStreamState::WaitForLogs, Some(block))
                }
            };
            self.state = next_state;
            if value_to_yield.is_some() { return Ok(Async::Ready(value_to_yield)); }
        }
    }
}
