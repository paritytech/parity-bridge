macro_rules! try_channel {
	($e: expr) => (match $e {
		Err(err) => return Err(From::from(err)),
		Ok(Async::NotReady) => None,
		Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
		Ok(Async::Ready(Some(value))) => Some(value),
	})
}

macro_rules! try_stream {
	($e: expr) => (match $e {
		Err(err) => return Err(From::from(err)),
		Ok(Async::NotReady) => return Ok(Async::NotReady),
		Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
		Ok(Async::Ready(Some(value))) => value,
	})
}
