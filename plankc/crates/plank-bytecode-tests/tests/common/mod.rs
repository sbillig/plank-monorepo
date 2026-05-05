pub mod abi;
pub mod erc20;
pub mod merkle_airdrop;
pub mod minimal_proxy;

use plank_bytecode_tests::{
    BackendCase, CallCase, PlankFixture, RuntimeExecution, run_runtime_case,
};

#[derive(Debug, Clone, Copy)]
pub struct RuntimeScenario {
    pub name: &'static str,
    pub fixture: fn() -> PlankFixture,
    pub constructor_args: fn() -> Vec<u8>,
    pub calls: fn() -> Vec<CallCase>,
}

pub const RUNTIME_SCENARIOS: [RuntimeScenario; 3] = [
    RuntimeScenario {
        name: "erc20-v1",
        fixture: erc20::fixture,
        constructor_args: erc20::constructor_args,
        calls: erc20::calls,
    },
    RuntimeScenario {
        name: "merkle-airdrop",
        fixture: merkle_airdrop::fixture,
        constructor_args: merkle_airdrop::constructor_args,
        calls: merkle_airdrop::calls,
    },
    RuntimeScenario {
        name: "minimal-proxy",
        fixture: minimal_proxy::fixture,
        constructor_args: minimal_proxy::constructor_args,
        calls: minimal_proxy::calls,
    },
];

pub fn run_runtime_scenario(
    scenario: &RuntimeScenario,
    backends: &[BackendCase],
) -> (Vec<CallCase>, Vec<RuntimeExecution>) {
    let calls = (scenario.calls)();
    let executions =
        run_runtime_case(&(scenario.fixture)(), &(scenario.constructor_args)(), &calls, backends)
            .unwrap_or_else(|err| panic!("{} runtime scenario should run: {err}", scenario.name));
    (calls, executions)
}
