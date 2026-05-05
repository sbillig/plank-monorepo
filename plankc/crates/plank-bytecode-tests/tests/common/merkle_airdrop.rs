use crate::common::abi::{address_word, selector_calldata};
use plank_bytecode_tests::{CallCase, PlankFixture, calldata_words};
use revm::primitives::{Address, U256, keccak256};

const SOURCE: &str = include_str!("../../../../plank-diff-tests/src/examples/merkle_airdrop.plk");
const CLAIM_SELECTOR: u32 = 0x3d13_f874;
const MERKLE_ROOT_SELECTOR: u32 = 0x2eb4_a7ab;
const HAS_CLAIMED_SELECTOR: u32 = 0x73b2_e80e;

pub const ALICE_AMOUNT: u64 = 100;
pub const BOB_AMOUNT: u64 = 200;
const CHARLIE_AMOUNT: u64 = 300;
const EVE_AMOUNT: u64 = 400;

pub fn fixture() -> PlankFixture {
    PlankFixture::new(SOURCE).with_std()
}

pub fn constructor_args() -> Vec<u8> {
    calldata_words([U256::from_be_slice(&tree().root)])
}

pub fn calls() -> Vec<CallCase> {
    vec![
        CallCase::new("merkle-root", merkle_root_calldata()),
        CallCase::new("has-claimed-alice-before", has_claimed_calldata(alice()))
            .expect_success_word(0),
        CallCase::new("claim-alice", claim_alice_calldata(ALICE_AMOUNT)).expect_success_empty(),
        CallCase::new("has-claimed-alice-after", has_claimed_calldata(alice()))
            .expect_success_word(1),
        CallCase::new("double-claim-alice", claim_alice_calldata(ALICE_AMOUNT)).expect_revert(),
        CallCase::new("claim-bob-wrong-amount", claim_bob_calldata(BOB_AMOUNT + 1)).expect_revert(),
    ]
}

pub fn alice() -> Address {
    Address::new([0xA1; 20])
}

pub fn bob() -> Address {
    Address::new([0xB0; 20])
}

pub fn merkle_root_calldata() -> Vec<u8> {
    selector_calldata(MERKLE_ROOT_SELECTOR, [])
}

pub fn has_claimed_calldata(address: Address) -> Vec<u8> {
    selector_calldata(HAS_CLAIMED_SELECTOR, [address_word(address)])
}

pub fn claim_alice_calldata(amount: u64) -> Vec<u8> {
    let tree = tree();
    claim_calldata(alice(), amount, [tree.leaf_bob, tree.node_cd])
}

pub fn claim_bob_calldata(amount: u64) -> Vec<u8> {
    let tree = tree();
    claim_calldata(bob(), amount, [tree.leaf_alice, tree.node_cd])
}

fn claim_calldata(address: Address, amount: u64, proof: [[u8; 32]; 2]) -> Vec<u8> {
    let mut calldata = selector_calldata(
        CLAIM_SELECTOR,
        [address_word(address), U256::from(amount), U256::from(96)],
    );
    calldata.extend_from_slice(&calldata_words([U256::from(proof.len())]));
    for element in proof {
        calldata.extend_from_slice(&element);
    }
    calldata
}

fn tree() -> MerkleTree {
    let leaf_alice = leaf(alice(), ALICE_AMOUNT);
    let leaf_bob = leaf(bob(), BOB_AMOUNT);
    let leaf_charlie = leaf(Address::new([0xC0; 20]), CHARLIE_AMOUNT);
    let leaf_eve = leaf(Address::new([0xE0; 20]), EVE_AMOUNT);
    let node_ab = hash_pair(leaf_alice, leaf_bob);
    let node_cd = hash_pair(leaf_charlie, leaf_eve);
    let root = hash_pair(node_ab, node_cd);
    MerkleTree { leaf_alice, leaf_bob, node_cd, root }
}

fn leaf(address: Address, amount: u64) -> [u8; 32] {
    let mut encoded = Vec::with_capacity(52);
    encoded.extend_from_slice(address.as_slice());
    encoded.extend_from_slice(&U256::from(amount).to_be_bytes::<32>());
    keccak256(encoded).into()
}

fn hash_pair(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    let mut encoded = Vec::with_capacity(64);
    if a <= b {
        encoded.extend_from_slice(&a);
        encoded.extend_from_slice(&b);
    } else {
        encoded.extend_from_slice(&b);
        encoded.extend_from_slice(&a);
    }
    keccak256(encoded).into()
}

struct MerkleTree {
    leaf_alice: [u8; 32],
    leaf_bob: [u8; 32],
    node_cd: [u8; 32],
    root: [u8; 32],
}
