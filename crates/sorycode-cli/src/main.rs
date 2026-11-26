use clap::{Parser, Subcommand};
use sorycode_application::SoryCodeApp;
use sorycode_core::ids::{ProjectId, SessionId};
use sorycode_core::model::Role;
use sorycode_events::{EngineEvent, EventEnvelope, EventSink};
use sorycode_orchestrator::Orchestrator;
use sorycode_protocol::{CoreCommand, CoreResponse};
use sorycode_providers::{PROVIDER_PRESETS, Provider, credentials, find_preset, model_catalog};
use sorycode_storage::Storage;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

mod input;
mod render;

const BANNER: &str = r#"
  ███████╗  ██████╗  ██████╗ ██╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗
  ██╔════╝ ██╔═══██╗ ██╔══██╗╚██╗ ██╔╝██╔════╝██╔═══██╗██╔══██╗██╔════╝
  ███████╗ ██║   ██║ ██████╔╝ ╚████╔╝ ██║     ██║   ██║██║  ██║█████╗
  ╚════██║ ██║   ██║ ██╔══██╗  ╚██╔╝  ██║     ██║   ██║██║  ██║██╔══╝
  ███████║ ╚██████╔╝ ██║  ██║   ██║   ╚██████╗╚██████╔╝██████╔╝███████╗
  ╚══════╝  ╚═════╝ ╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝
  SoryCode Core — agentic coding engine (Rust)
"#;

#[derive(Parser)]
#[command(name = "sorycode", version, about = "SoryCode Core CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Commands>,
    #[arg(long, default_value = ".")]
    dir: String,
    #[arg(long, env = "OPENAI_API_KEY")]
    api_key: Option<String>,
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long, default_value = "openrouter-free")]
    provider: String,
    /// Skip approvals and strict policy (auto-approve everything)
    #[arg(long)]
    yolo: bool,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(long)]
        prompt: String,
    },
    Init {
        #[arg(long, default_value = "project")]
        name: String,
    },
    Repl,
    Connect,
    Providers,
    Sessions,
    History {
        session_id: String,
    },
    Doctor,
}

struct PrintSink {
    /// Buffers streamed deltas so the final assistant message renders as a
    /// single boxed card instead of interleaved raw text.
    delta_buf: std::sync::Mutex<String>,
}
impl Default for PrintSink {
    fn default() -> Self {
        Self {
            delta_buf: std::sync::Mutex::new(String::new()),
        }
    }
}
impl EventSink for PrintSink {
    fn emit<'life0, 'async_trait>(
        &'life0 self,
        envelope: EventEnvelope,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            match envelope.event {
                EngineEvent::MessageDelta { delta, .. } => {
                    self.delta_buf
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push_str(&delta);
                }
                EngineEvent::ToolStarted { tool_name, .. } => {
                    eprintln!("{}", render::tool_line("start", &tool_name, "", None));
                }
                EngineEvent::ToolOutput { chunk, .. } => {
                    eprintln!(
                        "{}{}",
                        render::GRAY,
                        chunk.chars().take(500).collect::<String>()
                    );
                    eprintln!("{}", render::RESET);
                }
                EngineEvent::ToolCompleted {
                    is_error, summary, ..
                } => {
                    eprintln!(
                        "{}",
                        render::tool_line(
                            "done",
                            "tool",
                            &summary.chars().take(120).collect::<String>(),
                            Some(!is_error),
                        )
                    );
                }
                EngineEvent::TaskCompleted { result, .. } => {
                    let body = if result.trim().is_empty() {
                        self.delta_buf
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone()
                    } else {
                        result.clone()
                    };
                    self.delta_buf
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clear();
                    println!();
                    render::print_block(&render::assistant_card("assistant", &body, 60));
                }
                EngineEvent::TaskFailed { error, .. } => {
                    self.delta_buf
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clear();
                    render::print_block(&render::error_card("error", &error));
                }
                EngineEvent::ApprovalRequired { action, reason, .. } => {
                    eprintln!(
                        "\n\x1b[33m[approval required] {} — {}\x1b[0m",
                        action, reason
                    );
                }
                _ => {}
            }
        })
    }
}

fn build_app(
    storage: &Arc<Storage>,
    sink: &Arc<dyn EventSink>,
    provider: Arc<dyn Provider>,
    model: &str,
    dir: &str,
    yolo: bool,
) -> Arc<SoryCodeApp> {
    let mut orchestrator = Orchestrator::new(
        provider,
        sorycode_agent::AgentConfig {
            model: model.to_string(),
            strict_policy: !yolo,
            ..Default::default()
        },
    )
    .with_event_sink(sink.clone());
    if !yolo {
        orchestrator = orchestrator.with_approvals(Arc::new(CliApproval));
    }
    Arc::new(SoryCodeApp {
        storage: storage.clone(),
        orchestrator,
        event_sink: Some(sink.clone()),
        working_dir: dir.to_string(),
        task_cancels: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    })
}

/// Interactive approval handler: asks y/N on stderr+stdin for risky tool calls.
pub struct CliApproval;
#[async_trait::async_trait]
impl sorycode_agent::ApprovalHandler for CliApproval {
    async fn approve(&self, info: sorycode_security::ApprovalInfo) -> bool {
        let action = info.action.clone();
        let reason = info.reason.clone();
        tokio::task::spawn_blocking(move || {
            eprint!("\n{} ", render::approval_line(&action, &reason));
            let _ = io::stderr().flush();
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
        })
        .await
        .unwrap_or(false)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("warn").init();
    let cli = Cli::parse();
    let dir = cli.dir.clone();
    let yolo = cli.yolo;

    credentials::apply_to_env();

    let preset = find_preset(&cli.provider).unwrap_or_else(|| find_preset("openrouter-free").unwrap());
    let mut provider_name = preset.name.to_string();
    let mut model = cli
        .model
        .clone()
        .unwrap_or_else(|| preset.default_model.to_string());
    let provider: Arc<dyn Provider> =
        make_provider(&provider_name, cli.api_key.clone(), cli.base_url.clone()).0;

    let storage_path = std::path::Path::new(&dir)
        .join(".sorycode")
        .join("sorycode.db.json");
    let storage = Arc::new(Storage::connect_file(&storage_path).await?);

    let sink: Arc<dyn EventSink> = Arc::new(PrintSink::default());
    let mut app = build_app(&storage, &sink, provider, &model, &dir, yolo);

    match cli.cmd {
        Some(Commands::Init { name }) => {
            let now = chrono::Utc::now();
            let p = sorycode_core::project::Project {
                id: ProjectId::new(),
                name: name.clone(),
                root_path: dir.clone(),
                created_at: now,
                updated_at: now,
            };
            use sorycode_storage::ProjectRepository;
            storage.create(&p).await?;
            println!("project created: {}", p.id);
            Ok(())
        }
        Some(Commands::Run { prompt }) => run_one(&app, &prompt).await,
        Some(Commands::Connect) => {
            connect_flow_interactive().await;
            Ok(())
        }
        Some(Commands::Providers) => {
            for p in PROVIDER_PRESETS {
                let status = if std::env::var(p.env_key)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
                {
                    "configured"
                } else {
                    "not configured"
                };
                println!("{:<16} {}  ({})", p.name, status, p.env_key);
            }
            Ok(())
        }
        Some(Commands::Sessions) => {
            match app
                .handle(CoreCommand::ListSessions { project_id: None })
                .await
            {
                CoreResponse::Sessions { sessions } => {
                    if sessions.is_empty() {
                        println!("no sessions");
                    }
                    for s in sessions {
                        println!(
                            "{}  {}  {}",
                            s.id,
                            s.created_at.format("%Y-%m-%d %H:%M"),
                            s.title
                        );
                    }
                }
                other => println!("error: {:?}", other),
            }
            Ok(())
        }
        Some(Commands::History { session_id }) => {
            let sid: SessionId = session_id
                .trim()
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid session id: {}", e))?;
            match app
                .handle(CoreCommand::GetSessionHistory { session_id: sid })
                .await
            {
                CoreResponse::History { messages } => {
                    if messages.is_empty() {
                        println!("(empty history)");
                    }
                    for m in messages {
                        let role = match m.role {
                            Role::System => "system",
                            Role::User => "user",
                            Role::Assistant => "assistant",
                            Role::Tool => "tool",
                        };
                        let mut content = m.content.replace('\n', " ");
                        if content.chars().count() > 300 {
                            content = format!("{}…", content.chars().take(300).collect::<String>());
                        }
                        println!("[{}] {}", role, content);
                    }
                }
                other => println!("error: {:?}", other),
            }
            Ok(())
        }
        Some(Commands::Doctor) => {
            println!("dir: {}", dir);
            println!("storage: {}", storage_path.display());
            let writable = storage_path
                .parent()
                .map(|p| std::fs::create_dir_all(p).is_ok())
                .unwrap_or(false);
            println!("storage dir writable: {}", writable);
            for p in PROVIDER_PRESETS {
                let set = std::env::var(p.env_key)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false);
                println!(
                    "{:<16} {:<13} {}",
                    p.name,
                    if set { "configured" } else { "missing" },
                    p.env_key
                );
            }
            Ok(())
        }
        _ => {
            println!("{}", BANNER);
            println!(
                "  model: {} | provider: {} | dir: {}",
                model, provider_name, dir
            );
            if let Some(p) = find_preset(&provider_name) {
                let has_key = std::env::var(p.env_key)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false);
                if !has_key && !p.free_only {
                    eprintln!(
                        "  \x1b[33mwarning: no API key for {} — requests will fail.\x1b[0m",
                        provider_name
                    );
                    eprintln!(
                        "  \x1b[2mfree alternatives: /provider openrouter-free or /provider opencode-zen\x1b[0m"
                    );
                }
            }
            println!("  Type a request, or /help, /exit.\n");
            let mut session_id = match app
                .handle(CoreCommand::CreateSession {
                    project_id: None,
                    title: "repl".into(),
                })
                .await
            {
                CoreResponse::SessionCreated { session_id } => session_id,
                other => {
                    return Err(anyhow::anyhow!("session create failed: {:?}", other));
                }
            };
            let slash_commands: [(&str, &str); 16] = [
                ("/help", "Show commands"),
                ("/exit", "Quit"),
                ("/quit", "Quit"),
                ("/clear", "Clear screen"),
                ("/new", "New session"),
                ("/sessions", "List sessions"),
                ("/history", "Show session history"),
                ("/resume", "Resume a session by id"),
                ("/cleanhistory", "Clear session history"),
                ("/providers", "List providers"),
                ("/connect", "Configure an API key"),
                ("/model", "Pick a model (provider/model)"),
                ("/provider", "Pick a provider"),
                ("/plan", "Plan first, then execute after confirmation"),
                ("/allow", "Allow a shell command pattern (execpolicy)"),
                ("/deny", "Deny a shell command pattern (execpolicy)"),
            ];
            loop {
                let prompt = format!(
                    "\x1b[2m{} / {} · {}\x1b[0m \x1b[1;36m›\x1b[0m ",
                    provider_name,
                    model,
                    &session_id.to_string()[..8]
                );
                let read = input::read_line_interactive(&prompt, &slash_commands);
                let line: String = match read {
                    Ok(Some(l)) => l,
                    _ => break,
                };
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                let mut pending_execute: Option<String> = None;
                if input == "/exit" || input == "/quit" {
                    break;
                }
                if input == "/help" {
                    println!(
                        "Commands: /exit /new /sessions /history /resume <id> /cleanhistory /clear /model [name] /provider [name] /providers /connect"
                    );
                    continue;
                }
                if input == "/new" {
                    match app
                        .handle(CoreCommand::CreateSession {
                            project_id: None,
                            title: "repl".into(),
                        })
                        .await
                    {
                        CoreResponse::SessionCreated { session_id: sid } => {
                            session_id = sid;
                            println!("new session: {}", sid);
                        }
                        other => eprintln!("error: {:?}", other),
                    }
                    continue;
                }
                if input == "/sessions" {
                    match app
                        .handle(CoreCommand::ListSessions { project_id: None })
                        .await
                    {
                        CoreResponse::Sessions { sessions } => {
                            if sessions.is_empty() {
                                println!("no sessions");
                            }
                            for s in sessions {
                                println!(
                                    "{}  {}  {}",
                                    s.id,
                                    s.created_at.format("%Y-%m-%d %H:%M"),
                                    s.title
                                );
                            }
                        }
                        other => eprintln!("error: {:?}", other),
                    }
                    continue;
                }
                if input == "/history" {
                    match app
                        .handle(CoreCommand::GetSessionHistory { session_id })
                        .await
                    {
                        CoreResponse::History { messages } => {
                            if messages.is_empty() {
                                println!("(empty history)");
                            }
                            for m in messages {
                                let role = match m.role {
                                    Role::System => "system",
                                    Role::User => "user",
                                    Role::Assistant => "assistant",
                                    Role::Tool => "tool",
                                };
                                let mut content = m.content.replace('\n', " ");
                                if content.chars().count() > 200 {
                                    content = format!(
                                        "{}…",
                                        content.chars().take(200).collect::<String>()
                                    );
                                }
                                println!("[{}] {}", role, content);
                            }
                        }
                        other => eprintln!("error: {:?}", other),
                    }
                    continue;
                }
                if input == "/cleanhistory" {
                    match app
                        .handle(CoreCommand::ClearSessionHistory { session_id })
                        .await
                    {
                        CoreResponse::Ok => println!("history cleared"),
                        other => eprintln!("error: {:?}", other),
                    }
                    continue;
                }
                if let Some(rest) = input.strip_prefix("/resume ") {
                    match rest.trim().parse::<SessionId>() {
                        Ok(sid) => {
                            match app
                                .handle(CoreCommand::GetSession { session_id: sid })
                                .await
                            {
                                CoreResponse::Session { .. } => {
                                    session_id = sid;
                                    println!("resumed session {}", sid);
                                }
                                other => eprintln!("error: {:?}", other),
                            }
                        }
                        Err(e) => eprintln!("invalid session id: {}", e),
                    }
                    continue;
                }
                if input == "/providers" {
                    for p in PROVIDER_PRESETS {
                        let status = if std::env::var(p.env_key)
                            .map(|v| !v.is_empty())
                            .unwrap_or(false)
                        {
                            "configured"
                        } else {
                            "not configured"
                        };
                        println!("{:<16} {}  ({})", p.name, status, p.env_key);
                    }
                    continue;
                }
                if input == "/connect" {
                    connect_flow_interactive().await;
                    continue;
                }
                if input == "/provider" {
                    println!("Available providers (* = current):");
                    for (i, p) in PROVIDER_PRESETS.iter().enumerate() {
                        let status = if std::env::var(p.env_key)
                            .map(|v| !v.is_empty())
                            .unwrap_or(false)
                        {
                            "configured"
                        } else if p.free_only {
                            "free"
                        } else {
                            "no key"
                        };
                        let marker = if p.name == provider_name { " *" } else { "" };
                        println!(
                            "  {:>2}. {:<16} {:<11} {} ({}){}",
                            i + 1,
                            p.name,
                            status,
                            p.default_model,
                            p.env_key,
                            marker
                        );
                    }
                    let names: Vec<&str> = PROVIDER_PRESETS.iter().map(|p| p.name).collect();
                    match prompt_choice("select provider:", &names).await {
                        Some(choice) => {
                            if let Some(preset) =
                                PROVIDER_PRESETS.iter().find(|p| p.name == choice.as_str())
                            {
                                let new_name = preset.name;
                                if new_name != provider_name {
                                    let (new_provider, default_model) =
                                        make_provider(new_name, None, None);
                                    provider_name = new_name.to_string();
                                    model = default_model.to_string();
                                    app = build_app(
                                        &storage,
                                        &sink,
                                        new_provider,
                                        &model,
                                        &dir,
                                        yolo,
                                    );
                                    println!("Switched to provider: {} / {}", provider_name, model);
                                }
                            }
                        }
                        None => {}
                    }
                    continue;
                }
                if input == "/model" {
                    // Unified provider/model picker (Codex/OpenCode style)
                    let all = all_models();
                    println!("Available models (* = current):");
                    for (i, (pv, m)) in all.iter().enumerate() {
                        let marker = if *pv == provider_name && *m == model {
                            " *"
                        } else {
                            ""
                        };
                        println!("  {:>3}. {} / {}{}", i + 1, pv, m, marker);
                    }
                    let options: Vec<String> =
                        all.iter().map(|(pv, m)| format!("{}/{}", pv, m)).collect();
                    let opts: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
                    match prompt_choice("select model:", &opts).await {
                        Some(choice) => {
                            if let Some((pv, m)) = choice.split_once('/') {
                                switch_to_model(
                                    &mut app,
                                    &storage,
                                    &sink,
                                    &dir,
                                    &mut provider_name,
                                    &mut model,
                                    pv,
                                    m,
                                    yolo,
                                )
                                .await;
                            }
                        }
                        None => {}
                    }
                    continue;
                }
                if let Some(m) = input.strip_prefix("/model ") {
                    let arg = m.trim();
                    if let Some((pv, mv)) = arg.split_once('/') {
                        if PROVIDER_PRESETS.iter().any(|p| p.name == pv) {
                            switch_to_model(
                                &mut app,
                                &storage,
                                &sink,
                                &dir,
                                &mut provider_name,
                                &mut model,
                                pv,
                                mv,
                                yolo,
                            )
                            .await;
                        } else {
                            println!("Unknown provider in '{}'. Use /providers to list.", arg);
                        }
                    } else {
                        model = arg.to_string();
                        let prov = app.orchestrator.provider.clone();
                        app = build_app(&storage, &sink, prov, &model, &dir, yolo);
                        println!("Model set to: {} / {}", provider_name, model);
                    }
                    continue;
                }
                if let Some(p) = input.strip_prefix("/provider ") {
                    let new_name = p.trim();
                    if PROVIDER_PRESETS.iter().any(|pr| pr.name == new_name) {
                        let (new_provider, default_model) = make_provider(new_name, None, None);
                        provider_name = new_name.to_string();
                        model = default_model.to_string();
                        app = build_app(&storage, &sink, new_provider, &model, &dir, yolo);
                        println!("Switched to provider: {} / {}", provider_name, model);
                    } else {
                        println!("Unknown provider. Use /providers to list.");
                    }
                    continue;
                }
                if let Some(rest) = input.strip_prefix("/allow ") {
                    let pattern = rest.trim();
                    if !pattern.is_empty() {
                        match execpolicy_add(&dir, "allow", pattern) {
                            Ok(()) => println!("allowed pattern: {}", pattern),
                            Err(e) => eprintln!("error: {}", e),
                        }
                    }
                    continue;
                }
                if let Some(rest) = input.strip_prefix("/deny ") {
                    let pattern = rest.trim();
                    if !pattern.is_empty() {
                        match execpolicy_add(&dir, "deny", pattern) {
                            Ok(()) => println!("denied pattern: {}", pattern),
                            Err(e) => eprintln!("error: {}", e),
                        }
                    }
                    continue;
                }
                if input == "/clear" {
                    print!("\x1b[2J\x1b[H");
                    continue;
                }
                if let Some(task) = input.strip_prefix("/plan ") {
                    let task = task.trim();
                    if task.is_empty() {
                        println!("usage: /plan <task>");
                        continue;
                    }
                    let plan_req = sorycode_providers::ChatRequest {
                        model: model.clone(),
                        messages: vec![
                            sorycode_core::model::Message::system(
                                "You are a planning agent for a coding assistant. \
                                 Produce a concise step-by-step plan (bullets) to \
                                 accomplish the user's task. Do not execute anything.",
                            ),
                            sorycode_core::model::Message::user(task.to_string()),
                        ],
                        tools: vec![],
                        temperature: Some(0.2),
                        max_tokens: Some(1500),
                    };
                    match app.orchestrator.provider.chat(plan_req).await {
                        Ok(resp) => {
                            println!("\n\x1b[1mPlan:\x1b[0m\n{}", resp.content);
                            print!("Execute this plan? [y/N] ");
                            let _ = io::stdout().flush();
                            let stdin = tokio::io::stdin();
                            let mut reader = BufReader::new(stdin);
                            let mut ans = String::new();
                            let _ = reader.read_line(&mut ans).await;
                            if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                                pending_execute = Some(task.to_string());
                            } else {
                                println!("Plan discarded.");
                                continue;
                            }
                        }
                        Err(e) => {
                            eprintln!("plan failed: {}", e);
                            continue;
                        }
                    }
                }
                let resp = app
                    .handle(CoreCommand::SendMessage {
                        session_id,
                        content: pending_execute.take().unwrap_or_else(|| input.to_string()),
                    })
                    .await;
                let task_id = match resp {
                    CoreResponse::TaskStarted { task_id } => task_id,
                    other => {
                        eprintln!("error: {:?}", other);
                        continue;
                    }
                };
                let mut done = false;
                for _ in 0..600 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    match app.handle(CoreCommand::GetTaskStatus { task_id }).await {
                        CoreResponse::TaskStatus { status, .. } => {
                            use sorycode_core::task::TaskStatus;
                            match status {
                                TaskStatus::Completed
                                | TaskStatus::Failed
                                | TaskStatus::Cancelled => {
                                    done = true;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                if !done {
                    eprintln!("\n[timed out waiting for task]");
                }
                println!();
            }
            Ok(())
        }
    }
}

async fn run_one(app: &Arc<SoryCodeApp>, prompt: &str) -> anyhow::Result<()> {
    let session_id = match app
        .handle(CoreCommand::CreateSession {
            project_id: None,
            title: "run".into(),
        })
        .await
    {
        CoreResponse::SessionCreated { session_id } => session_id,
        other => {
            return Err(anyhow::anyhow!("session create failed: {:?}", other));
        }
    };
    let resp = app
        .handle(CoreCommand::SendMessage {
            session_id,
            content: prompt.to_string(),
        })
        .await;
    let task_id = match resp {
        CoreResponse::TaskStarted { task_id } => task_id,
        other => return Err(anyhow::anyhow!("send failed: {:?}", other)),
    };
    let mut done = false;
    for _ in 0..600 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match app.handle(CoreCommand::GetTaskStatus { task_id }).await {
            CoreResponse::TaskStatus { status, .. } => {
                use sorycode_core::task::TaskStatus;
                match status {
                    TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                        done = true;
                        break;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if !done {
        eprintln!("\n[timed out]");
    }
    Ok(())
}

/// Switch the active provider (if needed) and model, rebuilding the app.
async fn switch_to_model(
    app: &mut Arc<SoryCodeApp>,
    storage: &Arc<Storage>,
    sink: &Arc<dyn EventSink>,
    dir: &str,
    provider_name: &mut String,
    model: &mut String,
    provider: &str,
    new_model: &str,
    yolo: bool,
) {
    if provider != provider_name.as_str() {
        let (prov, _) = make_provider(provider, None, None);
        *provider_name = provider.to_string();
        *model = new_model.to_string();
        *app = build_app(storage, sink, prov, model, dir, yolo);
    } else {
        *model = new_model.to_string();
        let prov = app.orchestrator.provider.clone();
        *app = build_app(storage, sink, prov, model, dir, yolo);
    }
    println!("Switched to {} / {}", provider_name, model);
}

/// All known (provider, model) pairs across presets.
fn all_models() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for p in PROVIDER_PRESETS {
        for m in model_catalog(p.name) {
            out.push((p.name, *m));
        }
    }
    out
}

/// Append an allow/deny rule to `<dir>/.sorycode/execpolicy.json`.
fn execpolicy_add(dir: &str, action: &str, pattern: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(dir)
        .join(".sorycode")
        .join("execpolicy.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut rules: Vec<serde_json::Value> = if path.exists() {
        let data = std::fs::read_to_string(&path)?;
        serde_json::from_str::<serde_json::Value>(&data)
            .ok()
            .and_then(|v| v.get("rules").and_then(|r| r.as_array().cloned()))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // replace existing rule with same action+pattern
    rules.retain(|r| {
        !(r.get("action").and_then(|a| a.as_str()) == Some(action)
            && r.get("pattern").and_then(|p| p.as_str()) == Some(pattern))
    });
    rules.push(serde_json::json!({ "action": action, "pattern": pattern }));
    let out = serde_json::json!({ "rules": rules });
    std::fs::write(&path, serde_json::to_string_pretty(&out)?)?;
    Ok(())
}

/// Prompt the user to pick one of `options` (number or exact name).
async fn prompt_choice(prompt: &str, options: &[&str]) -> Option<String> {
    print!("{} ", prompt);
    let _ = io::stdout().flush();
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
        return None;
    }
    let choice = line.trim();
    if choice.is_empty() {
        return None;
    }
    if let Ok(n) = choice.parse::<usize>() {
        if n >= 1 && n <= options.len() {
            return Some(options[n - 1].to_string());
        }
        return None;
    }
    if options.contains(&choice) {
        return Some(choice.to_string());
    }
    None
}

fn make_provider(
    name: &str,
    api_key: Option<String>,
    base_url: Option<String>,
) -> (Arc<dyn Provider>, &'static str) {
    let preset = find_preset(name).unwrap_or_else(|| find_preset("openrouter-free").unwrap());
    let pname = preset.name;
    let key = api_key
        .or_else(|| std::env::var(preset.env_key).ok())
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| "public".to_string());
    let base = base_url.unwrap_or_else(|| preset.base_url.to_string());
    let default_model = preset.default_model;
    let provider: Arc<dyn Provider> = if pname == "gemini" {
        let mut p = sorycode_providers::GeminiProvider::new(key);
        p.base_url = base;
        Arc::new(p)
    } else {
        let mut prov = sorycode_providers::OpenAiCompatProvider::new(pname, base, key);
        if pname.starts_with("openrouter") {
            prov = prov.with_headers(vec![
                (
                    "HTTP-Referer".to_string(),
                    "https://sorycode.local".to_string(),
                ),
                ("X-Title".to_string(), "SoryCode".to_string()),
            ]);
        }
        if pname == "opencode-zen" {
            prov = prov.with_headers(vec![
                (
                    "x-opencode-session".to_string(),
                    "sorycode-session".to_string(),
                ),
                ("User-Agent".to_string(), "opencode/1.0.0".to_string()),
            ]);
        }
        Arc::new(prov)
    };
    (provider, default_model)
}

async fn connect_flow_interactive() {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    println!("\nAvailable providers:");
    for (i, p) in PROVIDER_PRESETS.iter().enumerate() {
        let status = if std::env::var(p.env_key)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            "configured"
        } else {
            "not configured"
        };
        println!("  {}. {:<16} {}  ({})", i + 1, p.name, status, p.env_key);
    }
    print!("\nSelect provider (number or name): ");
    let _ = io::stdout().flush();
    line.clear();
    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
        return;
    }
    let choice = line.trim();
    let preset = if let Ok(n) = choice.parse::<usize>() {
        if n >= 1 && n <= PROVIDER_PRESETS.len() {
            &PROVIDER_PRESETS[n - 1]
        } else {
            return;
        }
    } else {
        match PROVIDER_PRESETS.iter().find(|p| p.name == choice) {
            Some(p) => p,
            None => {
                println!("Unknown provider.");
                return;
            }
        }
    };

    println!("\nProvider: {} — {}", preset.name, preset.description);
    println!("Get your API key at the provider's console, then paste it here.");
    if preset.name == "opencode-zen" {
        println!("(OpenCode Zen free models work without a key; press Enter to skip.)");
    }
    print!("API key: ");
    let _ = io::stdout().flush();
    line.clear();
    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
        return;
    }
    let key = line.trim().to_string();

    if key.is_empty() && preset.name != "opencode-zen" {
        println!("No key provided; cancelled.");
        return;
    }
    let key_to_save = if key.is_empty() {
        "public".to_string()
    } else {
        key.clone()
    };
    let _ = credentials::save(preset.name, &key_to_save);
    unsafe {
        std::env::set_var(preset.env_key, &key_to_save);
    }
    println!("Saved. Provider {} is now configured.", preset.name);
    println!("Restart or use --provider {} to use it.", preset.name);
}
