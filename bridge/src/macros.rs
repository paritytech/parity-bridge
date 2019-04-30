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
/// like `try_ready!` but for streams
macro_rules! try_stream {
    ($e:expr) => {
        match $e {
            Err(err) => return Err(From::from(err)),
            Ok($crate::futures::Async::NotReady) => return Ok($crate::futures::Async::NotReady),
            Ok($crate::futures::Async::Ready(None)) => {
                return Ok($crate::futures::Async::Ready(None))
            }
            Ok($crate::futures::Async::Ready(Some(value))) => value,
        }
    };
}

/// like `try_stream` but returns `None` if `NotReady`
macro_rules! try_maybe_stream {
    ($e:expr) => {
        match $e {
            Err(err) => return Err(From::from(err)),
            Ok($crate::futures::Async::NotReady) => None,
            Ok($crate::futures::Async::Ready(None)) => {
                return Ok($crate::futures::Async::Ready(None))
            }
            Ok($crate::futures::Async::Ready(Some(value))) => Some(value),
        }
    };
}
