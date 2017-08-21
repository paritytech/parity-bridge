use web3::types::H256;
use ethabi;

use_contract!(mainnet, "EthereumBridge", "contracts/EthereumBridge.abi");
use_contract!(testnet, "KovanBridge", "contracts/KovanBridge.abi");

pub fn web3_topic(topic: ethabi::Topic<ethabi::Hash>) -> Option<Vec<H256>> {
	let t: Vec<ethabi::Hash> = topic.into();
	Some(t.into_iter().map(|x| H256(x)).collect())
}
