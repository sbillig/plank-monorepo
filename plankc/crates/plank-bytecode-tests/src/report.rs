use crate::{
    assertions::{CallExecution, RuntimeExecution},
    evm::BASE_TRANSACTION_GAS,
};
use std::{fmt::Write as _, io, path::PathBuf};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GasReport {
    pub rows: Vec<GasReportRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GasReportRow {
    pub scenario: String,
    pub backend: String,
    pub phase: String,
    pub initcode_bytes: usize,
    pub runtime_bytecode_bytes: usize,
    pub gas_used_minus_tx_base: u64,
}

impl GasReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_runtime(&mut self, scenario: impl Into<String>, execution: &RuntimeExecution) {
        let scenario = scenario.into();
        self.push_runtime_deploy(&scenario, execution);

        for call in &execution.calls {
            self.push_runtime_call(&scenario, execution, call);
        }
    }

    pub fn push_runtime_executions(
        &mut self,
        scenario: impl Into<String>,
        executions: &[RuntimeExecution],
    ) {
        let scenario = scenario.into();
        if executions.is_empty() {
            return;
        }

        let call_count = executions[0].calls.len();
        for execution in executions {
            assert_eq!(
                execution.calls.len(),
                call_count,
                "runtime gas report call count mismatch for {}",
                execution.backend.name
            );
            self.push_runtime_deploy(&scenario, execution);
        }

        for call_index in 0..call_count {
            for execution in executions {
                self.push_runtime_call(&scenario, execution, &execution.calls[call_index]);
            }
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut table = String::new();
        let _ = writeln!(
            table,
            "| scenario | backend | phase | init bytes | runtime bytes | gas used (-{} tx base) |",
            BASE_TRANSACTION_GAS
        );
        table.push_str("| --- | --- | --- | ---: | ---: | ---: |\n");
        for row in &self.rows {
            let _ = writeln!(
                table,
                "| {} | {} | {} | {} | {} | {} |",
                row.scenario,
                row.backend,
                row.phase,
                row.initcode_bytes,
                row.runtime_bytecode_bytes,
                row.gas_used_minus_tx_base
            );
        }
        table
    }

    pub fn to_json(&self) -> String {
        let mut json = format!(
            "{{\n  \"transaction_base_gas_subtracted\": {},\n  \"rows\": [\n",
            BASE_TRANSACTION_GAS
        );
        for (index, row) in self.rows.iter().enumerate() {
            if index > 0 {
                json.push_str(",\n");
            }
            let _ = write!(
                json,
                "    {{\"scenario\": {}, \"backend\": {}, \"phase\": {}, \
                 \"initcode_bytes\": {}, \"runtime_bytecode_bytes\": {}, \
                 \"gas_used_minus_tx_base\": {}}}",
                json_string(&row.scenario),
                json_string(&row.backend),
                json_string(&row.phase),
                row.initcode_bytes,
                row.runtime_bytecode_bytes,
                row.gas_used_minus_tx_base
            );
        }
        json.push_str("\n  ]\n}\n");
        json
    }

    pub fn write_json(&self, path: impl Into<PathBuf>) -> io::Result<()> {
        let path = path.into();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json())
    }

    fn push_runtime_deploy(&mut self, scenario: &str, execution: &RuntimeExecution) {
        self.rows.push(GasReportRow {
            scenario: scenario.to_string(),
            backend: execution.backend.name.to_string(),
            phase: "deploy".to_string(),
            initcode_bytes: execution.initcode_bytes,
            runtime_bytecode_bytes: execution.deploy.runtime_bytecode.len(),
            gas_used_minus_tx_base: subtract_tx_base(execution.deploy.init.gas_used),
        });
    }

    fn push_runtime_call(
        &mut self,
        scenario: &str,
        execution: &RuntimeExecution,
        call: &CallExecution,
    ) {
        self.rows.push(GasReportRow {
            scenario: scenario.to_string(),
            backend: execution.backend.name.to_string(),
            phase: format!("call:{}", call.name),
            initcode_bytes: execution.initcode_bytes,
            runtime_bytecode_bytes: execution.deploy.runtime_bytecode.len(),
            gas_used_minus_tx_base: subtract_tx_base(call.outcome.gas_used),
        });
    }
}

fn subtract_tx_base(gas_used: u64) -> u64 {
    gas_used
        .checked_sub(BASE_TRANSACTION_GAS)
        .expect("gas report rows should include transaction base gas")
}

fn json_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len() + 2);
    escaped.push('"');
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", ch as u32);
            }
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}
