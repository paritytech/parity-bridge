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

//! various helper functions

use error::{self, ResultExt};
use ethabi::{self, ContractFunction, ParseLog, RawLog};
use futures::future::FromErr;
use futures::{Async, Future, Poll, Stream};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serializer};
use std::time::Duration;
use tokio_timer::{Timeout, Timer};
use web3::api::Namespace;
use web3::helpers::CallResult;
use web3::types::{Address, Bytes, CallRequest, H256, TransactionRequest, U256};
use web3::{self, Transport};

/// attempts to convert a raw `web3_log` into the ethabi log type of a specific `event`
pub fn parse_log<T: ParseLog>(event: &T, web3_log: &web3::types::Log) -> ethabi::Result<T::Log> {
    let ethabi_log = RawLog {
        topics: web3_log.topics.iter().map(|t| t.0.into()).collect(),
        data: web3_log.data.0.clone(),
    };
    event.parse_log(ethabi_log)
}

/// use `AsyncCall::new(transport, contract_address, timeout, function)` to
/// get a `Future` that resolves with the decoded output from calling `function`
/// on `contract_address`.
pub struct AsyncCall<T: Transport, F: ContractFunction> {
    future: Timeout<FromErr<CallResult<Bytes, T::Out>, error::Error>>,
    function: F,
}

impl<T: Transport, F: ContractFunction> AsyncCall<T, F> {
    /// call `function` at `contract_address`.
    /// returns a `Future` that resolves with the decoded output of `function`.
    pub fn new(transport: &T, contract_address: Address, timeout: Duration, function: F) -> Self {
        let payload = function.encoded();
        let request = CallRequest {
            from: None,
            to: contract_address,
            gas: None,
            gas_price: None,
            value: None,
            data: Some(Bytes(payload)),
        };
        let inner_future = web3::api::Eth::new(transport)
            .call(request, None)
            .from_err();
        let future = Timer::default().timeout(inner_future, timeout);
        Self { future, function }
    }
}

impl<T: Transport, F: ContractFunction> Future for AsyncCall<T, F> {
    type Item = F::Output;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let encoded = try_ready!(
            self.future
                .poll()
                .chain_err(|| "failed to poll inner web3 CallResult future")
        );
        let decoded = self.function
            .output(encoded.0.clone())
            .chain_err(|| format!("failed to decode response {:?}", encoded))?;
        Ok(Async::Ready(decoded))
    }
}

pub struct AsyncTransaction<T: Transport> {
    future: Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>,
}

impl<T: Transport> AsyncTransaction<T> {
    pub fn new<F: ContractFunction>(
        transport: &T,
        contract_address: Address,
        authority_address: Address,
        gas: U256,
        gas_price: U256,
        timeout: Duration,
        f: F,
    ) -> Self {
        let request = TransactionRequest {
            from: authority_address,
            to: Some(contract_address),
            gas: Some(gas),
            gas_price: Some(gas_price),
            value: None,
            data: Some(Bytes(f.encoded())),
            nonce: None,
            condition: None,
        };
        let inner_future = web3::api::Eth::new(transport)
            .send_transaction(request)
            .from_err();
        let future = Timer::default().timeout(inner_future, timeout);
        Self { future }
    }
}

impl<T: Transport> Future for AsyncTransaction<T> {
    type Item = H256;
    type Error = error::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.future.poll().map_err(|x| x.into())
    }
}

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

/// extends the `Stream` trait by the `last` function
pub trait StreamExt<I> {
    /// if you're interested only in the last item in a stream
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
        Last {
            stream: self,
            last: None,
        }
    }
}

/// `Future` that wraps a `Stream` and completes with the last
/// item in the stream once the stream is over.
pub struct Last<S, I> {
    stream: S,
    last: Option<I>,
}

impl<S, I> Future for Last<S, I>
where
    S: Stream<Item = I>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures;
    use tokio_core::reactor::Core;

    #[test]
    fn test_stream_ext_last_empty() {
        let stream = futures::stream::empty::<(), ()>();
        let mut event_loop = Core::new().unwrap();
        assert_eq!(event_loop.run(stream.last()).unwrap(), None);
    }

    #[test]
    fn test_stream_ext_last_once_ok() {
        let stream = futures::stream::once::<u32, ()>(Ok(42));
        let mut event_loop = Core::new().unwrap();
        assert_eq!(event_loop.run(stream.last()).unwrap(), Some(42));
    }

    #[test]
    fn test_stream_ext_last_once_err() {
        let stream = futures::stream::once::<u32, u32>(Err(42));
        let mut event_loop = Core::new().unwrap();
        assert_eq!(event_loop.run(stream.last()).unwrap_err(), 42);
    }

    #[test]
    fn test_stream_ext_last_three() {
        let stream = futures::stream::iter_ok::<_, ()>(vec![17, 19, 3]);
        let mut event_loop = Core::new().unwrap();
        assert_eq!(event_loop.run(stream.last()).unwrap(), Some(3));
    }
}
