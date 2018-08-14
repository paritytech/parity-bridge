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
use futures::{Async, Future, Poll, Stream};

/// `OrderedStream` is a `Stream` that yields the
/// values of a list of `Future`s in a predefined order which is
/// independent from when the individual `Future`s complete.
///
/// let's say you `insert` a future **A** with order `4` into the ordered stream and a future **B** with order `2`.
/// even if **A** becomes available first the value of **B**
/// is yielded first because **B**s order is lower than **A**s:
///
/// ```
/// # extern crate tokio_core;
/// # extern crate tokio_timer;
/// # extern crate bridge;
/// # extern crate futures;
/// # use std::time::Duration;
/// # use bridge::OrderedStream;
/// # use futures::stream::Stream;
/// # use futures::Future;
/// let mut ordered_stream: OrderedStream<u32, futures::future::Join<tokio_timer::Sleep, futures::future::FutureResult<&str, tokio_timer::TimerError>>> = OrderedStream::new();
///
/// let timer = tokio_timer::Timer::default();
///
/// let a = timer.sleep(Duration::from_secs(1)).join(futures::future::ok("a"));
/// let b = timer.sleep(Duration::from_secs(2)).join(futures::future::ok("b"));
///
/// ordered_stream.insert(4, a);
/// ordered_stream.insert(2, b);
///
/// let mut event_loop = tokio_core::reactor::Core::new().unwrap();
///
/// let results = event_loop.run(ordered_stream.take(2).collect()).unwrap();
/// assert_eq!(results[0], (2, ((), "b")));
/// assert_eq!(results[1], (4, ((), "a")));
/// ```
///
/// items with the same `order` are yielded in the order they were `insert`ed.
///
/// example in the context of the bridge:
/// a `RelayStream` polls a Stream of logs
/// calls a ... for every log and yields the block number for
/// every block number 
/// by using the block number as `order`, 
/// and yielding the previous block number each time it changes
/// this is easily accomplished.
/// TODO

pub struct OrderedStream<O, F: Future> {
    entries: Vec<Entry<O, F>>,
}

impl<O: Ord, F: Future> OrderedStream<O, F> {
    /// returns a new empty `OrderedStream`
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// insert a `future` into this that should be yielded
    /// when it is completed and there are currently no
    /// futures inside the stream that have a smaller `order`.
    pub fn insert(&mut self, order: O, future: F) {
        self.entries.push(Entry {
            order,
            future,
            item_if_ready: None,
        });
    }

    /// returns the count of futures that have completed but can't be
    /// yielded since there are futures which are not ready
    pub fn ready_count(&self) -> usize {
        self.entries.iter().filter(|x| x.item_if_ready.is_some()).count()
    }

    /// returns the count of futures that have not yet completed
    pub fn not_ready_count(&self) -> usize {
        self.entries.iter().filter(|x| x.item_if_ready.is_none()).count()
    }
}

impl<O: Ord + Clone, F: Future> Stream for OrderedStream<O, F> {
    type Item = (O, F::Item);
    type Error = F::Error;

    /// `O(n)` where `n = self.entries.len()`.
    /// there's not much that can be done to improve this `O` since `poll` always must `poll` all `self.entries`.
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // minimum of orders of entries which are not ready
        let mut maybe_min_not_ready: Option<O> = None;
        // the index (in entries) of the completed order with the lowest order
        let mut maybe_min_ready: Option<(O, usize)> = None;

        for (index, entry) in self.entries.iter_mut().enumerate() {
            // poll futures which are not ready without every polling any future twice.
            if !entry.item_if_ready.is_some() {
                if let Async::Ready(item) = entry.future.poll()? {
                    entry.item_if_ready = Some(item);
                } else {
                    maybe_min_not_ready = maybe_min_not_ready
                        .map(|x| x.min(entry.order.clone()))
                        .or(Some(entry.order.clone()));
                }
            }

            if entry.item_if_ready.is_some() // item must be ready
                // we must initialize `maybe_min_ready`
                && (maybe_min_ready.is_none()
                // or entry is the new min
                || entry.order < maybe_min_ready.clone().expect("check in prev line. q.e.d.").0)
            {
                maybe_min_ready = Some((entry.order.clone(), index));
            }
        }

        if maybe_min_ready.is_none() {
            // there is no min ready -> none are ready
            return Ok(Async::NotReady);
        }

        let (min_ready_order, min_ready_index) =
            maybe_min_ready.expect("check and early return if none above. q.e.d.");

        if let Some(min_not_ready_order) = maybe_min_not_ready {
            // some are ready but there's unready ones with lower order
            if min_not_ready_order < min_ready_order {
                // there are futures which are not ready
                // but must be yielded before the ones that are ready
                // since their `order` is lower
                return Ok(Async::NotReady);
            }
        }

        // this is O(1)
        let entry_to_yield = self.entries.swap_remove(min_ready_index);

        Ok(Async::Ready(Some((
            entry_to_yield.order,
            entry_to_yield
                .item_if_ready
                .expect("`min_ready_index` points to index of entry with result. q.e.d."),
        ))))
    }
}

/// an entry in an `OrderedStream`
struct Entry<O, F: Future> {
    order: O,
    future: F,
    item_if_ready: Option<F::Item>,
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate tokio_core;
    extern crate tokio_timer;
    extern crate futures;
    use std::time::Duration;
    use futures::stream::Stream;
    use futures::Future;

    // TODO test multiple ready at same time
    //
    // TODO all are ready. none are not ready

    #[test]
    fn test_empty_ordered_stream_is_not_ready() {
        let mut ordered_stream: OrderedStream<u32, futures::future::Join<tokio_timer::Sleep, futures::future::FutureResult<&str, tokio_timer::TimerError>>> = OrderedStream::new();

        assert_eq!(ordered_stream.poll(), Ok(Async::NotReady));
        assert_eq!(ordered_stream.ready_count(), 0);
        assert_eq!(ordered_stream.not_ready_count(), 0);
    }

    #[test]
    fn test_single_insert() {
        let mut ordered_stream: OrderedStream<u32, futures::future::Join<tokio_timer::Sleep, futures::future::FutureResult<&str, tokio_timer::TimerError>>> = OrderedStream::new();

        assert_eq!(ordered_stream.poll(), Ok(Async::NotReady));
        assert_eq!(ordered_stream.ready_count(), 0);
        assert_eq!(ordered_stream.not_ready_count(), 0);

        let timer = tokio_timer::Timer::default();
        ordered_stream.insert(10, timer.sleep(Duration::from_millis(0)).join(futures::future::ok("f")));

        assert_eq!(ordered_stream.ready_count(), 0);
        assert_eq!(ordered_stream.not_ready_count(), 1);

        let mut event_loop = tokio_core::reactor::Core::new().unwrap();

        let stream_future = ordered_stream.into_future();
        let (item, ordered_stream) = if let Ok(success) = event_loop.run(stream_future) {
            success
        } else {
            panic!("failed to run stream_future");
        };

        assert_eq!(item, Some((10, ((), "f"))));

        assert_eq!(ordered_stream.ready_count(), 0);
        assert_eq!(ordered_stream.not_ready_count(), 0);
    }

    #[test]
    fn test_ordered_stream_7_insertions() {
        let mut ordered_stream: OrderedStream<u32, futures::future::Join<tokio_timer::Sleep, futures::future::FutureResult<&str, tokio_timer::TimerError>>> = OrderedStream::new();

        let timer = tokio_timer::Timer::default();

        ordered_stream.insert(10, timer.sleep(Duration::from_millis(0)).join(futures::future::ok("f")));
        ordered_stream.insert(4, timer.sleep(Duration::from_millis(1)).join(futures::future::ok("e")));
        ordered_stream.insert(3, timer.sleep(Duration::from_millis(65)).join(futures::future::ok("d")));
        ordered_stream.insert(0, timer.sleep(Duration::from_millis(500)).join(futures::future::ok("a")));
        ordered_stream.insert(2, timer.sleep(Duration::from_millis(50)).join(futures::future::ok("b")));
        ordered_stream.insert(2, timer.sleep(Duration::from_millis(10)).join(futures::future::ok("c")));
        ordered_stream.insert(10, timer.sleep(Duration::from_millis(338)).join(futures::future::ok("g")));

        assert_eq!(ordered_stream.ready_count(), 0);
        assert_eq!(ordered_stream.not_ready_count(), 7);

        let mut event_loop = tokio_core::reactor::Core::new().unwrap();

        let results = event_loop.run(ordered_stream.take(7).collect()).unwrap();
        assert_eq!(results, vec![
            (0, ((), "a")),
            (2, ((), "b")),
            (2, ((), "c")),
            (3, ((), "d")),
            (4, ((), "e")),
            (10, ((), "f")),
            (10, ((), "g")),
        ]);
    }
}
