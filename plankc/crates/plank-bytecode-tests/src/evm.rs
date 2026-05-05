use revm::{
    DatabaseCommit, ExecuteEvm, MainBuilder, MainContext,
    bytecode::Bytecode,
    context::{BlockEnv, Context, TxEnv},
    context_interface::result::ExecutionResult,
    database::CacheDB,
    database_interface::EmptyDB,
    primitives::{Address, Bytes, Log, StorageKey, StorageValue, TxKind, U256, hardfork::SpecId},
    state::{AccountInfo, EvmState},
};
use std::fmt::Debug;
use thiserror::Error;

pub const DEFAULT_CALLER: Address = Address::new([0xCA; 20]);
pub const DEFAULT_CONTRACT: Address = Address::new([0xCC; 20]);
pub const BASE_TRANSACTION_GAS: u64 = 21_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub status: ExecutionStatus,
    pub output: Vec<u8>,
    pub logs: Vec<Log>,
    pub storage_changes: Vec<StorageChange>,
    pub gas_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    Success,
    Revert,
    Halt(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Behavior {
    pub status: ExecutionStatus,
    pub output: Vec<u8>,
    pub logs: Vec<Log>,
    pub storage_changes: Vec<StorageChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StorageChange {
    pub address: Address,
    pub key: StorageKey,
    pub value: StorageValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployOutcome {
    pub address: Address,
    pub runtime_bytecode: Vec<u8>,
    pub init: ExecutionOutcome,
}

#[derive(Debug, Error)]
pub enum EvmError {
    #[error("failed to build EVM transaction: {0}")]
    Transaction(String),

    #[error("EVM execution failed: {0}")]
    Execution(String),
}

#[derive(Debug, Clone)]
pub struct EvmHarness {
    db: CacheDB<EmptyDB>,
    block: BlockEnv,
    caller: Address,
    nonce: u64,
    gas_limit: u64,
    spec: SpecId,
}

impl Default for EvmHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl EvmHarness {
    pub fn new() -> Self {
        let mut db = CacheDB::<EmptyDB>::default();
        db.insert_account_info(
            DEFAULT_CALLER,
            AccountInfo { balance: U256::MAX, ..AccountInfo::default() },
        );

        Self {
            db,
            block: BlockEnv::default(),
            caller: DEFAULT_CALLER,
            nonce: 0,
            gas_limit: 10_000_000,
            spec: SpecId::OSAKA,
        }
    }

    pub fn with_caller(mut self, caller: Address) -> Self {
        self.caller = caller;
        self.db.insert_account_info(
            caller,
            AccountInfo { balance: U256::MAX, ..AccountInfo::default() },
        );
        self.nonce = 0;
        self
    }

    pub fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas_limit = gas_limit;
        self.block.gas_limit = gas_limit;
        self
    }

    pub fn install_code(&mut self, address: Address, bytecode: &[u8]) {
        self.db.insert_account_info(
            address,
            AccountInfo::from_bytecode(Bytecode::new_raw(bytecode.to_vec().into())),
        );
    }

    pub fn execute_code(
        &mut self,
        address: Address,
        bytecode: &[u8],
        calldata: &[u8],
    ) -> Result<ExecutionOutcome, EvmError> {
        self.install_code(address, bytecode);
        self.call(address, calldata)
    }

    pub fn deploy(&mut self, initcode: &[u8]) -> Result<DeployOutcome, EvmError> {
        self.deploy_at(DEFAULT_CONTRACT, initcode)
    }

    pub fn deploy_at(
        &mut self,
        address: Address,
        initcode: &[u8],
    ) -> Result<DeployOutcome, EvmError> {
        let init = self.execute_code(address, initcode, &[])?;
        let runtime_bytecode = init.output.clone();
        if init.status == ExecutionStatus::Success {
            self.install_code(address, &runtime_bytecode);
        }
        Ok(DeployOutcome { address, runtime_bytecode, init })
    }

    pub fn call(
        &mut self,
        address: Address,
        calldata: &[u8],
    ) -> Result<ExecutionOutcome, EvmError> {
        let tx = TxEnv::builder()
            .caller(self.caller)
            .gas_limit(self.gas_limit)
            .gas_price(0)
            .kind(TxKind::Call(address))
            .data(Bytes::from(calldata.to_vec()))
            .nonce(self.nonce)
            .build()
            .map_err(|err| EvmError::Transaction(err.to_string()))?;

        self.run_tx(tx)
    }

    fn run_tx(&mut self, tx: TxEnv) -> Result<ExecutionOutcome, EvmError> {
        let mut evm = Context::mainnet()
            .with_db(self.db.clone())
            .with_block(self.block.clone())
            .modify_cfg_chained(|cfg| cfg.set_spec_and_mainnet_gas_params(self.spec))
            .build_mainnet();
        let result_and_state =
            evm.transact(tx).map_err(|err| EvmError::Execution(err.to_string()))?;
        let outcome =
            ExecutionOutcome::from_result(&result_and_state.result, &result_and_state.state);
        self.db.commit(result_and_state.state);
        self.nonce += 1;
        Ok(outcome)
    }
}

impl ExecutionOutcome {
    fn from_result<HaltReason: Debug>(
        result: &ExecutionResult<HaltReason>,
        state: &EvmState,
    ) -> Self {
        let (status, output, logs) = match result {
            ExecutionResult::Success { logs, output, .. } => {
                (ExecutionStatus::Success, output.data().to_vec(), logs.clone())
            }
            ExecutionResult::Revert { logs, output, .. } => {
                (ExecutionStatus::Revert, output.to_vec(), logs.clone())
            }
            ExecutionResult::Halt { reason, logs, .. } => {
                (ExecutionStatus::Halt(format!("{reason:?}")), Vec::new(), logs.clone())
            }
        };

        Self {
            status,
            output,
            logs,
            storage_changes: storage_changes(state),
            gas_used: result.gas_used(),
        }
    }

    pub fn behavior(&self) -> Behavior {
        Behavior {
            status: self.status.clone(),
            output: self.output.clone(),
            logs: self.logs.clone(),
            storage_changes: self.storage_changes.clone(),
        }
    }
}

pub fn word(value: U256) -> [u8; 32] {
    value.to_be_bytes()
}

pub fn calldata_words(words: impl IntoIterator<Item = U256>) -> Vec<u8> {
    let words = words.into_iter();
    let mut calldata = Vec::with_capacity(words.size_hint().0 * 32);
    for value in words {
        calldata.extend_from_slice(&word(value));
    }
    calldata
}

fn storage_changes(state: &EvmState) -> Vec<StorageChange> {
    let mut changes = Vec::new();
    for (address, account) in state {
        for (key, slot) in &account.storage {
            if slot.is_changed() {
                changes.push(StorageChange {
                    address: *address,
                    key: *key,
                    value: slot.present_value(),
                });
            }
        }
    }
    changes.sort();
    changes
}
