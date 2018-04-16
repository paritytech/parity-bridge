/// extraction of a pattern that occurred repeatedly in the codebase
///
/// where a "relay" is the detection of an event on chain A
/// followed by a transaction on chain B

use web3::types::Log;
use futures::{Future, Poll, Stream, Async};
use futures::future::{join_all, JoinAll};
use web3::Transport;
use log_stream::LogRange;
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
pub struct RelayStream<S: Stream<Item=LogRange, Error=error::Error>, F: RelayFactory> {
    log_stream: S,
    relay_factory: F,
    state: RelayStreamState<F::Relay>
}

impl<S: Stream<Item=LogRange, Error=error::Error>, F: RelayFactory> RelayStream<S, F> {
    pub fn new(
        log_stream: S,
        relay_factory: F,
    ) -> Self {
        Self {
            log_stream,
            relay_factory,
            state: RelayStreamState::WaitForLogs,
        }
    }
}

impl<S: Stream<Item=LogRange, Error=error::Error>, F: RelayFactory> Stream for RelayStream<S, F> {
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let (next_state, value_to_yield) = match self.state {
                RelayStreamState::WaitForLogs => {
                    let log_range = try_stream!(self.log_stream.poll());

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
