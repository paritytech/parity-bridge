use futures::{Stream, Future, Async, Poll};

// if you're interested in the side effects of the stream and not the values

pub struct Consume<S> where S: Stream {
    stream: S
}

impl<S> Consume<S>
    where S: Stream
{
    pub fn new(s: S) -> Self {
        Self {
            stream: s,
        }
    }
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
