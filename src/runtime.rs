use futures::Stream;

fn for_each_block_after_twelve_confirmations() -> Box<Stream<Item = (), Error = ()>> {
	unimplemented!();
};

fn run() {
	// get new block
	
	// wait 12 confirmations
	for_each_block_after_twelve_confirmations()
		.map(|block| {
			// if block matches bloom filter
			if true {
				// schedule work for each log
			}
			()
		});

	unimplemented!();
	// get logs for that block
	
	// process them (send transactions)
	// save transactions and their hashes in database
	
	// mark block as processed one
}
