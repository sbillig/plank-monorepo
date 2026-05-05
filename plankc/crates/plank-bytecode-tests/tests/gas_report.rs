#![allow(unused_crate_dependencies)]

mod common;

use common::{RUNTIME_SCENARIOS, run_runtime_scenario};
use plank_bytecode_tests::{GAS_BACKENDS, GasReport, assert_runtime_calls_match};
use std::path::Path;

#[test]
#[ignore = "report-only gas and bytecode-size comparison"]
fn print_gas_report() {
    let mut report = GasReport::new();

    for scenario in &RUNTIME_SCENARIOS {
        let (_, executions) = run_runtime_scenario(scenario, &GAS_BACKENDS);
        assert_runtime_calls_match(&executions);
        report.push_runtime_executions(scenario.name, &executions);
    }

    println!("{}", report.to_markdown());
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/plank-bytecode-tests/gas-report.json");
    report.write_json(path.clone()).expect("gas report should be written");
    println!("wrote {}", path.display());
}
