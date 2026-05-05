use crate::common::abi::{address_word, selector_calldata};
use plank_bytecode_tests::{CallCase, PlankFixture};
use revm::primitives::{Address, U256};

const SOURCE: &str = include_str!("../../../../plank-diff-tests/src/examples/minimal_proxy.plk");
const CLONE_SELECTOR: u32 = 0x8124_b78e;
const CLONE_DETERMINISTIC_SELECTOR: u32 = 0xb86b_2ceb;

pub fn fixture() -> PlankFixture {
    PlankFixture::new(SOURCE).with_std()
}

pub fn constructor_args() -> Vec<u8> {
    Vec::new()
}

pub fn calls() -> Vec<CallCase> {
    vec![
        CallCase::new("clone", clone_calldata()).expect_success_non_zero_word(),
        CallCase::new("clone-deterministic", clone_deterministic_calldata(U256::from(42)))
            .expect_success_non_zero_word(),
    ]
}

pub fn implementation() -> Address {
    Address::new([0x1D; 20])
}

pub fn clone_calldata() -> Vec<u8> {
    selector_calldata(CLONE_SELECTOR, [address_word(implementation())])
}

pub fn clone_deterministic_calldata(salt: U256) -> Vec<u8> {
    selector_calldata(CLONE_DETERMINISTIC_SELECTOR, [address_word(implementation()), salt])
}
