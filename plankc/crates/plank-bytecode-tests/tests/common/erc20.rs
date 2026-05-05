use crate::common::abi::{abi_bytes, address_word, selector_calldata};
use plank_bytecode_tests::{CallCase, DEFAULT_CALLER, PlankFixture};
use revm::primitives::{Address, U256};

const SOURCE: &str = include_str!("../../../../plank-diff-tests/src/examples/erc20.plk");
const BALANCE_OF_SELECTOR: u32 = 0x70a0_8231;
const TOTAL_SUPPLY_SELECTOR: u32 = 0x1816_0ddd;
const ALLOWANCE_SELECTOR: u32 = 0xdd62_ed3e;
const TRANSFER_SELECTOR: u32 = 0xa905_9cbb;
const APPROVE_SELECTOR: u32 = 0x095e_a7b3;
const TRANSFER_FROM_SELECTOR: u32 = 0x23b8_72dd;
const DECIMALS_SELECTOR: u32 = 0x313c_e567;
const NAME_SELECTOR: u32 = 0x06fd_de03;
const SYMBOL_SELECTOR: u32 = 0x95d8_9b41;

pub const TOTAL_SUPPLY: u64 = 1_000_000;
pub const TRANSFER_AMOUNT: u64 = 37;
pub const APPROVE_AMOUNT: u64 = 11;
pub const TRANSFER_FROM_AMOUNT: u64 = 5;

pub fn fixture() -> PlankFixture {
    PlankFixture::new(SOURCE).with_std()
}

pub fn recipient() -> Address {
    Address::new([0xBB; 20])
}

pub fn calls() -> Vec<CallCase> {
    let recipient = recipient();
    vec![
        CallCase::new("total-supply", total_supply_calldata()).expect_success_word(TOTAL_SUPPLY),
        CallCase::new("decimals", decimals_calldata()).expect_success_word(18),
        CallCase::new("name", name_calldata()).expect_success_bytes(name_output()),
        CallCase::new("symbol", symbol_calldata()).expect_success_bytes(symbol_output()),
        CallCase::new("balance-owner-before", balance_of_calldata(DEFAULT_CALLER))
            .expect_success_word(TOTAL_SUPPLY),
        CallCase::new("allowance-self-before", allowance_calldata(DEFAULT_CALLER, DEFAULT_CALLER))
            .expect_success_word(0),
        CallCase::new("approve-self", approve_calldata(DEFAULT_CALLER, U256::from(APPROVE_AMOUNT)))
            .expect_success_word(1),
        CallCase::new("allowance-self-after", allowance_calldata(DEFAULT_CALLER, DEFAULT_CALLER))
            .expect_success_word(APPROVE_AMOUNT),
        CallCase::new(
            "transfer-to-recipient",
            transfer_calldata(recipient, U256::from(TRANSFER_AMOUNT)),
        )
        .expect_success_word(1),
        CallCase::new("balance-owner-after", balance_of_calldata(DEFAULT_CALLER))
            .expect_success_word(TOTAL_SUPPLY - TRANSFER_AMOUNT),
        CallCase::new("balance-recipient-after", balance_of_calldata(recipient))
            .expect_success_word(TRANSFER_AMOUNT),
        CallCase::new(
            "transfer-from-self",
            transfer_from_calldata(DEFAULT_CALLER, recipient, U256::from(TRANSFER_FROM_AMOUNT)),
        )
        .expect_success_word(1),
        CallCase::new("balance-recipient-after-transfer-from", balance_of_calldata(recipient))
            .expect_success_word(TRANSFER_AMOUNT + TRANSFER_FROM_AMOUNT),
        CallCase::new(
            "transfer-too-much",
            transfer_calldata(recipient, U256::from(TOTAL_SUPPLY + 1)),
        )
        .expect_revert(),
    ]
}

pub fn constructor_args() -> Vec<u8> {
    Vec::new()
}

pub fn balance_of_calldata(owner: Address) -> Vec<u8> {
    selector_calldata(BALANCE_OF_SELECTOR, [address_word(owner)])
}

pub fn total_supply_calldata() -> Vec<u8> {
    selector_calldata(TOTAL_SUPPLY_SELECTOR, [])
}

pub fn allowance_calldata(owner: Address, spender: Address) -> Vec<u8> {
    selector_calldata(ALLOWANCE_SELECTOR, [address_word(owner), address_word(spender)])
}

pub fn transfer_calldata(to: Address, amount: U256) -> Vec<u8> {
    selector_calldata(TRANSFER_SELECTOR, [address_word(to), amount])
}

pub fn approve_calldata(spender: Address, amount: U256) -> Vec<u8> {
    selector_calldata(APPROVE_SELECTOR, [address_word(spender), amount])
}

pub fn transfer_from_calldata(from: Address, to: Address, amount: U256) -> Vec<u8> {
    selector_calldata(TRANSFER_FROM_SELECTOR, [address_word(from), address_word(to), amount])
}

pub fn decimals_calldata() -> Vec<u8> {
    selector_calldata(DECIMALS_SELECTOR, [])
}

pub fn name_calldata() -> Vec<u8> {
    selector_calldata(NAME_SELECTOR, [])
}

pub fn symbol_calldata() -> Vec<u8> {
    selector_calldata(SYMBOL_SELECTOR, [])
}

pub fn name_output() -> Vec<u8> {
    abi_bytes(b"PlankToken")
}

pub fn symbol_output() -> Vec<u8> {
    abi_bytes(b"PLK")
}
