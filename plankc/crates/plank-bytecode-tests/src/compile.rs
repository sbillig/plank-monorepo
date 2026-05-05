use plank_driver::{BackendKind, Driver};
use plank_source::source_fs::InMemoryFs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendCase {
    pub name: &'static str,
    pub backend: BackendKind,
    pub optimizations: Option<&'static str>,
}

pub const SIR_CSUD: BackendCase =
    BackendCase { name: "sir-csud", backend: BackendKind::Sir, optimizations: Some("csud") };
pub const SONA_O2: BackendCase =
    BackendCase { name: "sona-o2", backend: BackendKind::Sona, optimizations: Some("o2") };

pub const GAS_BACKENDS: [BackendCase; 2] = [SIR_CSUD, SONA_O2];

#[derive(Debug, Clone)]
pub struct PlankFixture {
    entry_path: PathBuf,
    files: Vec<FixtureFile>,
    modules: Vec<ModuleRoot>,
    std_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct FixtureFile {
    path: PathBuf,
    content: String,
}

#[derive(Debug, Clone)]
struct ModuleRoot {
    name: String,
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledContract {
    pub backend: BackendCase,
    pub bytecode: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("compilation failed:\n{diagnostics}")]
    Diagnostics { diagnostics: String },

    #[error("{backend} bytecode emission failed: {message}")]
    Backend { backend: &'static str, message: String },
}

impl PlankFixture {
    pub fn new(source: impl Into<String>) -> Self {
        Self::with_entry("main.plk", source)
    }

    pub fn with_entry(path: impl Into<PathBuf>, source: impl Into<String>) -> Self {
        let entry_path = path.into();
        Self {
            files: vec![FixtureFile { path: entry_path.clone(), content: source.into() }],
            entry_path,
            modules: Vec::new(),
            std_root: None,
        }
    }

    pub fn add_file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.push(FixtureFile { path: path.into(), content: content.into() });
        self
    }

    pub fn add_module(mut self, name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        self.modules.push(ModuleRoot { name: name.into(), root: root.into() });
        self
    }

    pub fn with_std(mut self) -> Self {
        let root = PathBuf::from("/std");
        for (path, content) in STD_FILES {
            self.files.push(FixtureFile { path: root.join(path), content: (*content).to_string() });
        }
        self.std_root = Some(root);
        self
    }

    pub fn compile(&self, backend: BackendCase) -> Result<CompiledContract, CompileError> {
        let mut fs = InMemoryFs::new();
        for file in &self.files {
            fs.add_file(&file.path, file.content.clone());
        }

        let mut driver = Driver::new(&fs);
        if let Some(root) = &self.std_root {
            driver.register_std(root.clone());
        }
        for module in &self.modules {
            driver.register_module(&module.name, module.root.clone());
        }

        let Some(project) = driver.load_project(Path::new(&self.entry_path)) else {
            return Err(CompileError::Diagnostics { diagnostics: render_diagnostics(&driver) });
        };
        if driver.session.has_errors() {
            return Err(CompileError::Diagnostics { diagnostics: render_diagnostics(&driver) });
        }

        let hir = driver.lower_hir(&project);
        if driver.session.has_errors() {
            return Err(CompileError::Diagnostics { diagnostics: render_diagnostics(&driver) });
        }

        let mir = driver.evaluate_hir(&hir, project.core_ops_source);
        if driver.session.has_errors() {
            return Err(CompileError::Diagnostics { diagnostics: render_diagnostics(&driver) });
        }

        let bytecode = driver
            .emit_bytecode_with_backend(
                &mir,
                backend.optimizations,
                false,
                false,
                false,
                backend.backend,
            )
            .map_err(|message| CompileError::Backend { backend: backend.name, message })?;

        Ok(CompiledContract { backend, bytecode })
    }

    pub fn compile_all(
        &self,
        backends: &[BackendCase],
    ) -> Result<Vec<CompiledContract>, CompileError> {
        backends.iter().map(|backend| self.compile(*backend)).collect()
    }
}

const STD_FILES: [(&str, &str); 13] = [
    ("abi.plk", include_str!("../../../../std/abi.plk")),
    ("abi_helpers.plk", include_str!("../../../../std/abi_helpers.plk")),
    ("constructor.plk", include_str!("../../../../std/constructor.plk")),
    ("core_ops.plk", include_str!("../../../../std/core_ops.plk")),
    ("math.plk", include_str!("../../../../std/math.plk")),
    ("mem.plk", include_str!("../../../../std/mem.plk")),
    ("membytes.plk", include_str!("../../../../std/membytes.plk")),
    ("option.plk", include_str!("../../../../std/option.plk")),
    ("sol.plk", include_str!("../../../../std/sol.plk")),
    ("storage.plk", include_str!("../../../../std/storage.plk")),
    ("string.plk", include_str!("../../../../std/string.plk")),
    ("type.plk", include_str!("../../../../std/type.plk")),
    ("utils.plk", include_str!("../../../../std/utils.plk")),
];

fn render_diagnostics<F: plank_source::source_fs::SourceFs>(driver: &Driver<'_, F>) -> String {
    driver
        .session
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.render_plain(&driver.session))
        .collect::<Vec<_>>()
        .join("\n\n")
}
