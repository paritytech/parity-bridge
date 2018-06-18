/// solves the following problem:
///
/// you have a list of futures that gets added
/// you want to get the results back in an order that is
/// defined before the
///
/// example:
///
/// only in ascending order of blocks
///
/// OrderedStream can
///
/// if there are no elements with a lower order in the stream
///
/// doctest
///
/// specific order in which they should be output
///
/// a bit like a binary heap
///
/// TODO[snd] possibly add more efficient implementation later

/// that complete at different times but should be output
/// in a specific order regardless of when they complete
/// futures which are associated with
use futures::{Async, Future, Poll, Stream};

struct Entry<O, F: Future> {
    order: O,
    future: F,
    result: Option<F::Item>,
}

pub struct FutureHeap<O, F: Future> {
    entries: Vec<Entry<O, F>>,
}

impl<O: Ord, F: Future> FutureHeap<O, F> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, order: O, future: F) {
        self.entries.push(Entry {
            order,
            future,
            result: None,
        });
    }
}

impl<O: Ord + Clone, F: Future> Stream for FutureHeap<O, F> {
    type Item = (O, F::Item);
    type Error = F::Error;

    /// `O(n)` where `n = self.entries.len()`
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut maybe_min_not_ready_order: Option<O> = None;
        let mut maybe_min_ready_order: Option<O> = None;
        let mut maybe_index_of_min_ready: Option<usize> = None;

        for (index, entry) in self.entries.iter_mut().enumerate() {
            // poll futures which aren't resolved yet
            if !entry.result.is_some() {
                match entry.future.poll()? {
                    Async::Ready(result) => {
                        entry.result = Some(result);
                    }
                    Async::NotReady => {}
                }
            }

            if entry.result.is_some() {
                if let Some(order) = maybe_min_ready_order.clone() {
                    if entry.order < order {
                        maybe_min_ready_order = Some(entry.order.clone());
                        maybe_index_of_min_ready = Some(index);
                    }
                } else {
                    maybe_min_ready_order = Some(entry.order.clone());
                    maybe_index_of_min_ready = Some(index);
                }
            } else {
                maybe_min_not_ready_order =
                    maybe_min_not_ready_order.map(|x| x.min(entry.order.clone()));
            }
        }

        if maybe_min_ready_order.is_none() {
            // there is no min ready -> none are ready
            return Ok(Async::NotReady);
        }

        let min_ready_order =
            maybe_min_ready_order.expect("check and early return if none above. q.e.d.");
        let index_of_min_ready = maybe_index_of_min_ready
            .expect("always set with `maybe_min_ready_order` above. q.e.d.");

        if let Some(min_not_ready_order) = maybe_min_not_ready_order {
            if min_not_ready_order < min_ready_order {
                // there are futures which are not ready and should come before
                return Ok(Async::NotReady);
            }
        }

        // this is O(1)
        let entry = self.entries.swap_remove(index_of_min_ready);

        Ok(Async::Ready(Some((
            entry.order,
            entry
                .result
                .expect("`index_of_min_ready` points to index of entry with result. q.e.d."),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_future_heap() {
        // TODO test multiple ready at same time
        //
        // TODO all are ready. none are not ready
    }
}
