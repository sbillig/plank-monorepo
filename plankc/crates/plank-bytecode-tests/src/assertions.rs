use crate::{
    compile::{BackendCase, CompileError, PlankFixture},
    evm::{DeployOutcome, EvmError, EvmHarness, ExecutionOutcome, ExecutionStatus},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallCase {
    pub name: String,
    pub calldata: Vec<u8>,
    pub expected: ExpectedOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedOutcome {
    AnySuccess,
    SuccessEmpty,
    SuccessWord(u64),
    SuccessBytes(Vec<u8>),
    SuccessNonZeroWord,
    Revert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExecution {
    pub backend: BackendCase,
    pub initcode_bytes: usize,
    pub deploy: DeployOutcome,
    pub calls: Vec<CallExecution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallExecution {
    pub name: String,
    pub outcome: ExecutionOutcome,
}

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error(transparent)]
    Compile(#[from] CompileError),

    #[error(transparent)]
    Evm(#[from] EvmError),
}

impl CallCase {
    pub fn new(name: impl Into<String>, calldata: impl Into<Vec<u8>>) -> Self {
        Self { name: name.into(), calldata: calldata.into(), expected: ExpectedOutcome::AnySuccess }
    }

    pub fn expect(mut self, expected: ExpectedOutcome) -> Self {
        self.expected = expected;
        self
    }

    pub fn expect_success_word(self, expected: u64) -> Self {
        self.expect(ExpectedOutcome::SuccessWord(expected))
    }

    pub fn expect_success_bytes(self, expected: impl Into<Vec<u8>>) -> Self {
        self.expect(ExpectedOutcome::SuccessBytes(expected.into()))
    }

    pub fn expect_success_empty(self) -> Self {
        self.expect(ExpectedOutcome::SuccessEmpty)
    }

    pub fn expect_success_non_zero_word(self) -> Self {
        self.expect(ExpectedOutcome::SuccessNonZeroWord)
    }

    pub fn expect_revert(self) -> Self {
        self.expect(ExpectedOutcome::Revert)
    }
}

pub fn run_runtime_case(
    fixture: &PlankFixture,
    constructor_args: &[u8],
    calls: &[CallCase],
    backends: &[BackendCase],
) -> Result<Vec<RuntimeExecution>, HarnessError> {
    let mut executions = Vec::with_capacity(backends.len());
    for backend in backends {
        let compiled = fixture.compile(*backend)?;
        let mut initcode = compiled.bytecode;
        initcode.extend_from_slice(constructor_args);
        let mut evm = EvmHarness::new();
        let deploy = evm.deploy(&initcode)?;
        let mut call_executions = Vec::with_capacity(calls.len());

        if deploy.init.status == ExecutionStatus::Success {
            for call in calls {
                let outcome = evm.call(deploy.address, &call.calldata)?;
                call_executions.push(CallExecution { name: call.name.clone(), outcome });
            }
        }

        executions.push(RuntimeExecution {
            backend: *backend,
            initcode_bytes: initcode.len(),
            deploy,
            calls: call_executions,
        });
    }
    Ok(executions)
}

#[track_caller]
pub fn assert_runtime_calls_match(executions: &[RuntimeExecution]) {
    assert!(!executions.is_empty(), "expected at least one backend execution");

    let baseline = &executions[0];
    let mut deploy_behavior = baseline.deploy.init.behavior();
    deploy_behavior.output.clear();
    let call_count = baseline.calls.len();

    for execution in &executions[1..] {
        let mut execution_deploy_behavior = execution.deploy.init.behavior();
        execution_deploy_behavior.output.clear();
        assert_eq!(
            execution_deploy_behavior, deploy_behavior,
            "deploy behavior mismatch for {} versus {}",
            execution.backend.name, baseline.backend.name
        );
        assert_eq!(
            execution.calls.len(),
            call_count,
            "runtime call count mismatch for {} versus {}",
            execution.backend.name,
            baseline.backend.name
        );
    }

    for index in 0..call_count {
        let baseline_call = &baseline.calls[index];
        let baseline_behavior = baseline_call.outcome.behavior();
        for execution in &executions[1..] {
            let call = &execution.calls[index];
            assert_eq!(
                call.name, baseline_call.name,
                "runtime call name mismatch for {} call {index}",
                execution.backend.name
            );
            assert_eq!(
                call.outcome.behavior(),
                baseline_behavior,
                "runtime call '{}' behavior mismatch for {} versus {}",
                baseline_call.name,
                execution.backend.name,
                baseline.backend.name
            );
        }
    }
}

#[track_caller]
pub fn assert_runtime_expectations(executions: &[RuntimeExecution], calls: &[CallCase]) {
    for execution in executions {
        assert_eq!(
            execution.deploy.init.status,
            ExecutionStatus::Success,
            "{} deploy should succeed",
            execution.backend.name
        );
        assert!(
            !execution.deploy.runtime_bytecode.is_empty(),
            "{} deploy should produce runtime bytecode",
            execution.backend.name
        );
        assert_eq!(
            execution.calls.len(),
            calls.len(),
            "{} runtime call count should match scenario",
            execution.backend.name
        );
        for (actual, expected) in execution.calls.iter().zip(calls) {
            assert_eq!(actual.name, expected.name);
            expected.expected.assert_matches(&actual.outcome, &actual.name, execution.backend.name);
        }
    }
}

impl ExpectedOutcome {
    #[track_caller]
    fn assert_matches(&self, outcome: &ExecutionOutcome, call: &str, backend: &str) {
        match self {
            Self::AnySuccess => assert_success(outcome, call, backend),
            Self::SuccessEmpty => {
                assert_success(outcome, call, backend);
                assert!(
                    outcome.output.is_empty(),
                    "{backend} call '{call}' should return no bytes"
                );
            }
            Self::SuccessWord(expected) => {
                assert_success(outcome, call, backend);
                assert_eq!(
                    outcome.output.as_slice(),
                    word_from_u64(*expected),
                    "{backend} call '{call}'"
                );
            }
            Self::SuccessBytes(expected) => {
                assert_success(outcome, call, backend);
                assert_eq!(&outcome.output, expected, "{backend} call '{call}'");
            }
            Self::SuccessNonZeroWord => {
                assert_success(outcome, call, backend);
                assert_eq!(outcome.output.len(), 32, "{backend} call '{call}'");
                assert_ne!(outcome.output.as_slice(), [0; 32], "{backend} call '{call}'");
            }
            Self::Revert => assert_eq!(
                outcome.status,
                ExecutionStatus::Revert,
                "{backend} call '{call}' should revert"
            ),
        }
    }
}

#[track_caller]
fn assert_success(outcome: &ExecutionOutcome, call: &str, backend: &str) {
    assert_eq!(outcome.status, ExecutionStatus::Success, "{backend} call '{call}' should succeed");
}

fn word_from_u64(value: u64) -> [u8; 32] {
    let mut word = [0; 32];
    word[24..].copy_from_slice(&value.to_be_bytes());
    word
}
