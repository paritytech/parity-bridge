use serde::{Deserialize, Deserializer, Serializer};
use serde::de::Error;
use ethereum_types::U256;
use futures::{Stream, Future, Async, Poll};

/// the toml crate parses integer literals as `i64`.
/// certain config options (example: `max_total_home_contract_balance`)
/// frequently don't fit into `i64`.
/// workaround: put them in string literals, use this custom
/// deserializer and parse them as U256.
pub fn deserialize_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    U256::from_dec_str(s).map_err(|_| D::Error::custom("failed to parse U256 from dec str"))
}

pub fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    serializer.serialize_str(&format!("{}", value))
}

pub trait StreamExt {
// if you're interested in the side effects of the stream and not the values
    fn consume(self) -> Consume<Self> where Self: Sized;
}

impl<S> StreamExt for S where S: Stream {
    fn consume(self) -> Consume<Self> where Self: Sized {
        Consume { stream: self }
    }
}

pub struct Consume<S> {
    stream: S
}

impl<S> Future for Consume<S>
    where S: Stream
{
    type Item = ();
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Self::Item, S::Error> {
        loop {
            match self.stream.poll() {
                Err(err) => return Err(err),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                // stream is finished
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                // there's more. ignore values
                Ok(Async::Ready(Some(_))) => {},
            }
        }
    }
}
