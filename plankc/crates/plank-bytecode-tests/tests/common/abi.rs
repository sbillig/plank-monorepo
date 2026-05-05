use plank_bytecode_tests::{calldata_words, word};
use revm::primitives::{Address, U256};

pub fn selector_calldata(selector: u32, args: impl IntoIterator<Item = U256>) -> Vec<u8> {
    let args = args.into_iter();
    let mut calldata = Vec::with_capacity(4 + args.size_hint().0 * 32);
    calldata.extend_from_slice(&selector.to_be_bytes());
    calldata.extend_from_slice(&calldata_words(args));
    calldata
}

pub fn address_word(address: Address) -> U256 {
    U256::from_be_slice(address.as_slice())
}

pub fn abi_bytes(value: &[u8]) -> Vec<u8> {
    let padded_len = value.len().next_multiple_of(32);
    let mut encoded = Vec::with_capacity(32 + padded_len);
    encoded.extend_from_slice(&word(U256::from(value.len())));
    encoded.extend_from_slice(value);
    encoded.resize(32 + padded_len, 0);
    encoded
}
