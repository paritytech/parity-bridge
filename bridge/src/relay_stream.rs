/// extraction of a pattern that occurred repeatedly in the codebase

use web3::types::Log;
use futures::{Future, Poll, Stream, Async};
use futures::future::{join_all, JoinAll};
use web3::Transport;
use log_stream::LogStream;
use error;

/// a thing that can create relay futures from logs
pub trait RelayFactory {
    type Relay;
    fn log_to_relay(&self, log: Log) -> Self::Relay;
}

/// state of the state machine that is the relay stream
enum RelayStreamState<T> {
    WaitForLogs,
    WaitForRelays {
        future: JoinAll<Vec<T>>,
        block: u64,
    },
}

/// a tokio `Stream` that when polled fetches all new `HomeBridge.Deposit`
/// events from `logs` and for each of them executes a `ForeignBridge.deposit`
/// transaction and waits for the configured confirmations.
/// stream yields last block on `home` for which all `HomeBrige.Deposit`
/// events have been handled this way.
pub struct RelayStream<T, F> {
    logs: LogStream<T>,
    relay_factory: F,
    state: RelayStreamState<T>
}

impl<T: Transport + Clone, F: RelayFactory, R: Future> RelayStream<T> {
    pub fn new(
        logs: LogStream<T>,
        relay_factory: RelayFactory<Relay = R>,
    ) -> Self {
        Self {
            logs,
            relay_factory,
            state: RelayStreamState::WaitForLogs,
        }
    }
}

impl<T: Transport> Stream for RelayStream<T> {
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
