use serde::{Deserialize, Deserializer, Serializer};
use serde::de::Error;
use ethereum_types::U256;
use futures::{Async, Future, Poll, Stream};

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

pub fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{}", value))
}

pub trait StreamExt<I> {
    // if you're interested only in the last item in a stream
    fn last(self) -> Last<Self, I>
    where
        Self: Sized;
}

impl<S, I> StreamExt<I> for S
where
    S: Stream,
{
    fn last(self) -> Last<Self, I>
    where
        Self: Sized,
    {
        Last { stream: self, last: None }
    }
}

pub struct Last<S, I> {
    stream: S,
    last: Option<I>,
}

impl<S, I> Future for Last<S, I>
where
    S: Stream<Item = I>
{
    type Item = Option<I>;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Self::Item, S::Error> {
        loop {
            match self.stream.poll() {
                Err(err) => return Err(err),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                // stream is finished
                Ok(Async::Ready(None)) => return Ok(Async::Ready(self.last.take())),
                // there is more
                Ok(Async::Ready(item)) => self.last = item,
            }
        }
    }
}
