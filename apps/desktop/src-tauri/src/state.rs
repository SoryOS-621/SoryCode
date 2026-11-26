//! Engine state owned by the Tauri shell.
//!
//! The state wraps `SoryCodeApp` and can rebuild the provider/orchestrator when
//! the user selects another provider or model. It contains no business logic:
//! every operation is delegated to `sorycode-application`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sorycode_agent::AgentConfig;
use sorycode_application::SoryCodeApp;
use sorycode_events::EventSink;
use sorycode_orchestrator::Orchestrator;
use sorycode_providers::{create_provider, find_preset, model_catalog, Provider, PROVIDER_PRESETS};
use sorycode_storage::Storage;
use tokio::sync::RwLock;

use crate::approvals::TauriApproval;
use crate::ipc::{SelectionDto, WorkspaceDto};
use crate::system::{credentials, paths, settings};

struct EngineInner {
    app: Arc<SoryCodeApp>,
    provider: String,
    model: String,
}

pub struct EngineState {
    inner: RwLock<EngineInner>,
    storage: Arc<Storage>,
    sink: Arc<dyn EventSink>,
    working_dir: RwLock<PathBuf>,
    /// The project the user explicitly opened, if any. `None` means no
    /// project is open: the file explorer stays empty and the agent works
    /// inside the isolated scratch directory.
    project_root: RwLock<Option<PathBuf>>,
    prefs: RwLock<settings::Settings>,
    approvals: Arc<TauriApproval>,
}

impl EngineState {
    pub async fn new(
        sink: Arc<dyn EventSink>,
        approvals: Arc<TauriApproval>,
    ) -> Result<Self, String> {
        let project = settings::selected_workspace();
        let workspace = project.clone().unwrap_or_else(paths::scratch_dir);
        std::fs::create_dir_all(&workspace).map_err(|error| {
            format!("cannot create working directory {}: {error}", workspace.display())
        })?;
        let storage = Arc::new(
            Storage::connect_file(&paths::data_path())
                .await
                .map_err(|error| format!("cannot open engine storage: {error}"))?,
        );
        let prefs = settings::load();
        let provider = prefs
            .provider
            .clone()
            .filter(|name| find_preset(name).is_some())
            .unwrap_or_else(default_provider);
        let model = prefs
            .model
            .clone()
            .unwrap_or_else(|| default_model(&provider));

        let inner = build_inner(
            storage.clone(),
            sink.clone(),
            workspace.to_string_lossy().to_string(),
            provider,
            model,
            approvals.clone(),
        )?;

        Ok(Self {
            inner: RwLock::new(inner),
            storage,
            sink,
            working_dir: RwLock::new(workspace),
            project_root: RwLock::new(project),
            prefs: RwLock::new(prefs),
            approvals,
        })
    }

    pub async fn app(&self) -> Arc<SoryCodeApp> {
        self.inner.read().await.app.clone()
    }

    pub async fn storage(&self) -> Arc<Storage> {
        self.storage.clone()
    }

    pub fn approvals(&self) -> Arc<TauriApproval> {
        self.approvals.clone()
    }

    /// The open project, or `None` when the user has not selected one.
    pub async fn project_root(&self) -> Option<PathBuf> {
        self.project_root.read().await.clone()
    }

    pub async fn workspace(&self) -> Option<WorkspaceDto> {
        self.project_root()
            .await
            .map(|path| workspace_dto(&path))
    }

    pub async fn selection(&self) -> SelectionDto {
        let inner = self.inner.read().await;
        selection_dto(&inner.provider, &inner.model)
    }

    pub async fn select_provider(&self, provider: &str) -> Result<SelectionDto, String> {
        if find_preset(provider).is_none() {
            return Err(format!("unknown provider: {provider}"));
        }
        let current = self.inner.read().await.provider.clone();
        let model = if current == provider {
            self.inner.read().await.model.clone()
        } else {
            default_model(provider)
        };
        self.rebuild(provider.to_string(), model).await
    }

    pub async fn select_model(&self, model: &str) -> Result<SelectionDto, String> {
        let model = model.trim();
        if model.is_empty() {
            return Err("model name cannot be empty".into());
        }
        let provider = self.inner.read().await.provider.clone();
        self.rebuild(provider, model.to_string()).await
    }

    pub async fn connect_provider(
        &self,
        provider: &str,
        api_key: &str,
    ) -> Result<SelectionDto, String> {
        let preset = find_preset(provider)
            .ok_or_else(|| format!("unknown provider: {provider}"))?;
        if api_key.trim().is_empty() && !preset.free_only {
            return Err(format!("{provider} requires an API key"));
        }
        let key = if api_key.trim().is_empty() {
            "public"
        } else {
            api_key.trim()
        };
        credentials::save(provider, key)
            .map_err(|error| format!("cannot save credentials: {error}"))?;

        let (current, current_model) = {
            let inner = self.inner.read().await;
            (inner.provider.clone(), inner.model.clone())
        };
        if current == provider {
            self.rebuild(provider.to_string(), current_model).await
        } else {
            Ok(self.selection().await)
        }
    }

    pub async fn set_workspace(&self, path: &str) -> Result<WorkspaceDto, String> {
        let canonical = paths::canonical_dir(path)?;
        *self.working_dir.write().await = canonical.clone();
        *self.project_root.write().await = Some(canonical.clone());
        {
            let mut prefs = self.prefs.write().await;
            prefs.workspace = Some(canonical.to_string_lossy().to_string());
            settings::save(&prefs)?;
        }
        self.rebuild_current().await?;
        Ok(workspace_dto(&canonical))
    }

    async fn rebuild(&self, provider: String, model: String) -> Result<SelectionDto, String> {
        let workspace = self.working_dir.read().await.clone();
        let inner = build_inner(
            self.storage.clone(),
            self.sink.clone(),
            workspace.to_string_lossy().to_string(),
            provider.clone(),
            model.clone(),
            self.approvals.clone(),
        )?;

        {
            let mut prefs = self.prefs.write().await;
            prefs.provider = Some(provider.clone());
            prefs.model = Some(model.clone());
            settings::save(&prefs)?;
        }
        *self.inner.write().await = inner;
        Ok(selection_dto(&provider, &model))
    }

    async fn rebuild_current(&self) -> Result<SelectionDto, String> {
        let inner = self.inner.read().await;
        let provider = inner.provider.clone();
        let model = inner.model.clone();
        drop(inner);
        self.rebuild(provider, model).await
    }
}

fn build_inner(
    storage: Arc<Storage>,
    sink: Arc<dyn EventSink>,
    working_dir: String,
    provider: String,
    model: String,
    approvals: Arc<TauriApproval>,
) -> Result<EngineInner, String> {
    let preset = find_preset(&provider).ok_or_else(|| format!("unknown provider: {provider}"))?;
    let api_key = credentials::resolve(&provider).unwrap_or_default();
    let provider_impl: Arc<dyn Provider> = Arc::from(
        create_provider(preset.name, &api_key)
            .ok_or_else(|| format!("cannot build provider: {provider}"))?,
    );
    let orchestrator = Orchestrator::new(
        provider_impl,
        AgentConfig {
            model: model.clone(),
            // Ask-mode by default, like the CLI without --yolo: writes/exec/
            // network go through TauriApproval -> frontend dialog.
            strict_policy: true,
            ..Default::default()
        },
    )
    .with_event_sink(sink.clone())
    .with_approvals(approvals);

    let app = Arc::new(SoryCodeApp {
        storage,
        orchestrator,
        event_sink: Some(sink),
        working_dir,
        task_cancels: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    });

    Ok(EngineInner {
        app,
        provider,
        model,
    })
}

fn default_provider() -> String {
    PROVIDER_PRESETS
        .iter()
        .find(|preset| credentials::resolve(preset.name).is_some())
        .map(|preset| preset.name.to_string())
        .unwrap_or_else(|| "opencode-zen".to_string())
}

fn default_model(provider: &str) -> String {
    find_preset(provider)
        .map(|preset| preset.default_model.to_string())
        .or_else(|| model_catalog(provider).first().map(|model| model.to_string()))
        .unwrap_or_else(|| "gpt-4o-mini".to_string())
}

fn selection_dto(provider: &str, model: &str) -> SelectionDto {
    let mut models: Vec<String> = model_catalog(provider)
        .iter()
        .map(|item| item.to_string())
        .collect();
    if !models.iter().any(|item| item == model) {
        models.insert(0, model.to_string());
    }
    SelectionDto {
        provider: provider.to_string(),
        model: model.to_string(),
        models,
    }
}

fn workspace_dto(path: &Path) -> WorkspaceDto {
    WorkspaceDto {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        is_git: path.join(".git").exists(),
        git_branch: crate::system::workspace::git_branch(path),
    }
}
