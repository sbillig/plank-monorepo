#![allow(unused_crate_dependencies)]

mod common;

use common::{RUNTIME_SCENARIOS, run_runtime_scenario};
use plank_bytecode_tests::{GAS_BACKENDS, assert_runtime_calls_match, assert_runtime_expectations};

#[test]
fn comparison_scenarios_execute_on_sir_and_sona() {
    for scenario in &RUNTIME_SCENARIOS {
        let (calls, executions) = run_runtime_scenario(scenario, &GAS_BACKENDS);
        assert_runtime_calls_match(&executions);
        assert_runtime_expectations(&executions, &calls);
    }
}
