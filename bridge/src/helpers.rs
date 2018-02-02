use ethereum_types;
use web3;

pub fn u256_to_web3(value: ethereum_types::U256) -> web3::types::U256 {
	let mut bytes = [0u8; 32];
	value.to_little_endian(&mut bytes[..]);
	web3::types::U256::from(&bytes[..])
}

// TODO test this
