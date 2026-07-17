//! Official multi-provider UX helpers.
//!
//! Grok Build already supports custom models via `~/.grok/config.toml` `[model.*]`
//! (see user-guide `11-custom-models.md`). This module only **reads/writes that
//! same file** — it does not invent a second provider runtime.

use std::path::{Path, PathBuf};

use crate::config_toml_edit::read_config_document_for_edit;
use clap::{Parser, Subcommand};

/// Official docs path (in-repo) for custom models.
pub const CUSTOM_MODELS_DOC: &str =
    "crates/codegen/xai-grok-pager/docs/user-guide/11-custom-models.md";

/// CLI: `dgrok provider …` and shared logic for `/provider`.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "provider",
    about = "Manage custom models in ~/.grok/config.toml (official multi-provider config)"
)]
pub struct ProviderArgs {
    #[command(subcommand)]
    pub command: Option<ProviderCommand>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProviderCommand {
    /// List `[model.*]` entries in user config.toml
    List,
    /// Show builtin presets that can be written into config.toml
    Presets,
    /// Write a preset or custom model into config.toml
    Add {
        /// Preset id (nvidia, anthropic, openai, ollama, openrouter) or free-form id
        id: String,
        /// Override base_url
        #[arg(long)]
        base_url: Option<String>,
        /// Override model id sent to the API
        #[arg(long)]
        model: Option<String>,
        /// Env var name for the API key (preferred over pasting secrets)
        #[arg(long)]
        env_key: Option<String>,
        /// API backend: chat_completions | responses | messages
        #[arg(long)]
        api_backend: Option<String>,
        /// Set [models].default to this id after writing
        #[arg(long)]
        set_default: bool,
        /// Overwrite an existing [model.<id>] table
        #[arg(long)]
        force: bool,
    },
    /// Interactive setup: list endpoints → key → fetch models → write config
    Setup,
    /// Write api_key onto existing [model.*] (fixes 401 when env_key is unset)
    SetKey {
        /// Only models with this env_key (default: OPENCODE_API_KEY)
        #[arg(long)]
        env_key: Option<String>,
        /// Or match base_url substring (e.g. opencode.ai / nvidia.com)
        #[arg(long)]
        base_url_contains: Option<String>,
        /// API key value (omit to prompt interactively)
        #[arg(long)]
        key: Option<String>,
    },
    /// List curated endpoints (OpenCode, NVIDIA, Anthropic, Codex, …)
    Endpoints,
    /// Print short usage + pointer to official docs
    Guide,
}

/// One official-style model block (fields match ConfigModelOverride / docs).
#[derive(Debug, Clone)]
pub struct ModelPreset {
    pub id: &'static str,
    pub model: &'static str,
    pub base_url: &'static str,
    pub name: &'static str,
    pub env_key: Option<&'static str>,
    pub api_backend: Option<&'static str>,
    pub notes: &'static str,
}

/// Built-in presets — only templates for the official config format.
pub fn builtin_presets() -> &'static [ModelPreset] {
    &[
        ModelPreset {
            id: "nvidia",
            model: "meta/llama-3.1-70b-instruct",
            base_url: "https://integrate.api.nvidia.com/v1",
            name: "NVIDIA NIM",
            env_key: Some("NVIDIA_API_KEY"),
            api_backend: Some("chat_completions"),
            notes: "OpenAI-compatible NIM. Set model= to a NIM id (e.g. z-ai/glm-5.2). export NVIDIA_API_KEY before launch.",
        },
        ModelPreset {
            id: "opencode",
            model: "mimo-v2.5-free",
            base_url: "https://opencode.ai/zen/v1",
            name: "OpenCode Zen",
            env_key: Some("OPENCODE_API_KEY"),
            api_backend: Some("chat_completions"),
            notes: "base_url is .../zen/v1 only (no /chat/completions). API model id is bare (mimo-v2.5-free), not opencode/….",
        },
        ModelPreset {
            id: "openai",
            model: "gpt-4o",
            base_url: "https://api.openai.com/v1",
            name: "OpenAI",
            env_key: Some("OPENAI_API_KEY"),
            api_backend: Some("chat_completions"),
            notes: "Standard Chat Completions.",
        },
        ModelPreset {
            id: "anthropic",
            model: "claude-opus-4-6",
            base_url: "https://api.anthropic.com/v1",
            name: "Anthropic Claude",
            env_key: Some("ANTHROPIC_API_KEY"),
            api_backend: Some("messages"),
            notes: "Uses api_backend=messages. Prefer env_key; extra_headers may still be needed for x-api-key — see 11-custom-models.md.",
        },
        ModelPreset {
            id: "ollama",
            model: "llama3.1",
            base_url: "http://127.0.0.1:11434/v1",
            name: "Ollama (local)",
            env_key: None,
            api_backend: Some("chat_completions"),
            notes: "Local Ollama OpenAI-compatible endpoint.",
        },
        ModelPreset {
            id: "openrouter",
            model: "openai/gpt-4o",
            base_url: "https://openrouter.ai/api/v1",
            name: "OpenRouter",
            env_key: Some("OPENROUTER_API_KEY"),
            api_backend: Some("chat_completions"),
            notes: "OpenRouter OpenAI-compatible gateway.",
        },
        ModelPreset {
            id: "codex",
            model: "gpt-5.4",
            base_url: "https://api.openai.com/v1",
            name: "OpenAI Codex / Responses",
            env_key: Some("OPENAI_API_KEY"),
            api_backend: Some("responses"),
            notes: "Responses API. No browser OAuth in dgrok — use OPENAI_API_KEY. Prefer: dgrok provider setup.",
        },
        ModelPreset {
            id: "groq",
            model: "llama-3.3-70b-versatile",
            base_url: "https://api.groq.com/openai/v1",
            name: "Groq",
            env_key: Some("GROQ_API_KEY"),
            api_backend: Some("chat_completions"),
            notes: "Groq OpenAI-compatible.",
        },
        ModelPreset {
            id: "deepseek",
            model: "deepseek-chat",
            base_url: "https://api.deepseek.com/v1",
            name: "DeepSeek",
            env_key: Some("DEEPSEEK_API_KEY"),
            api_backend: Some("chat_completions"),
            notes: "DeepSeek OpenAI-compatible.",
        },
    ]
}

/// Strip path suffixes users paste from docs (Grok appends the route itself).
///
/// `https://opencode.ai/zen/v1/chat/completions` → `https://opencode.ai/zen/v1`
pub fn normalize_provider_base_url(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('/').to_string();
    for suffix in ["/chat/completions", "/messages", "/responses"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.trim_end_matches('/').to_string();
        }
    }
    s
}

/// OpenCode internal ids use `opencode/<id>`; the Zen HTTP API wants bare `<id>`.
pub fn normalize_provider_model_id(model: &str) -> String {
    let m = model.trim();
    m.strip_prefix("opencode/")
        .or_else(|| m.strip_prefix("opencode-go/"))
        .unwrap_or(m)
        .to_string()
}

pub fn user_config_toml_path() -> PathBuf {
    xai_grok_config::grok_home().join("config.toml")
}

fn normalize_backend(s: &str) -> Option<&'static str> {
    match s.trim().to_ascii_lowercase().as_str() {
        "chat_completions" | "chat" | "openai" => Some("chat_completions"),
        "responses" | "response" => Some("responses"),
        "messages" | "anthropic" | "claude" => Some("messages"),
        _ => None,
    }
}

/// List `[model.<id>]` keys present in the document.
pub fn list_model_ids(doc: &toml_edit::DocumentMut) -> Vec<String> {
    let mut ids = Vec::new();
    // Nested or dotted tables: [model.foo]
    if let Some(item) = doc.get("model") {
        if let Some(table) = item.as_table() {
            for (k, v) in table.iter() {
                if v.is_table() || v.is_inline_table() || v.is_value() {
                    // value-only shouldn't count; require map-like
                    if v.as_table().is_some() || v.as_inline_table().is_some() {
                        ids.push(k.to_string());
                    }
                }
            }
        }
    }
    // Also scan root for dotted keys like "model.nvidia" tables (implicit form)
    for (key, item) in doc.iter() {
        if let Some(rest) = key.strip_prefix("model.") {
            if item.as_table().is_some() || item.as_inline_table().is_some() {
                ids.push(rest.to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

/// Write or update `[model.<id>]` with official fields only.
pub fn upsert_model_table(
    path: &Path,
    id: &str,
    model: &str,
    base_url: &str,
    name: Option<&str>,
    env_key: Option<&str>,
    api_backend: Option<&str>,
    set_default: bool,
    force: bool,
) -> Result<String, String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "model id must be non-empty ASCII alphanumeric, '-' or '_' (config table key)".into(),
        );
    }
    let model = normalize_provider_model_id(model);
    let base_url = normalize_provider_base_url(base_url);
    if base_url.is_empty() {
        return Err("base_url must not be empty after normalization".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let Some(mut doc) = read_config_document_for_edit(path) else {
        return Err(format!(
            "refusing to edit unparseable config at {} — fix syntax first",
            path.display()
        ));
    };

    let exists = list_model_ids(&doc).iter().any(|k| k == id);
    if exists && !force {
        return Err(format!(
            "[model.{id}] already exists — pass --force to overwrite, or pick another id"
        ));
    }

    // Official format: [model.<id>] standard tables (not inline maps),
    // matching user-guide 11-custom-models.md and ConfigModelOverride parsing.
    if doc.get("model").and_then(|i| i.as_table()).is_none() {
        let mut root = toml_edit::Table::new();
        root.set_implicit(true);
        doc["model"] = toml_edit::Item::Table(root);
    } else if let Some(root) = doc["model"].as_table_mut() {
        root.set_implicit(true);
    }

    let mut entry = toml_edit::Table::new();
    entry["model"] = toml_edit::value(&model);
    entry["base_url"] = toml_edit::value(&base_url);
    if let Some(n) = name {
        entry["name"] = toml_edit::value(n);
    }
    if let Some(ek) = env_key {
        entry["env_key"] = toml_edit::value(ek);
    }
    if let Some(backend) = api_backend {
        let b = normalize_backend(backend).ok_or_else(|| {
            format!("unknown api_backend `{backend}` (use chat_completions|responses|messages)")
        })?;
        entry["api_backend"] = toml_edit::value(b);
    }
    doc["model"][id] = toml_edit::Item::Table(entry);

    if set_default {
        if doc.get("models").and_then(|i| i.as_table()).is_none() {
            doc["models"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        doc["models"]["default"] = toml_edit::value(id);
    }

    std::fs::write(path, doc.to_string()).map_err(|e| e.to_string())?;
    let mut msg = format!(
        "Wrote [model.{id}] to {}\n  model={model}\n  base_url={base_url}{}\nUse: dgrok -m {id}   or in TUI: /model {id}\nDocs: {CUSTOM_MODELS_DOC}",
        path.display(),
        env_key
            .map(|e| format!("\n  env_key={e}  (export this env var BEFORE starting dgrok; do not put secrets in TOML)"))
            .unwrap_or_default()
    );
    if let Some(ek) = env_key {
        let set = std::env::var(ek).map(|v| !v.trim().is_empty()).unwrap_or(false);
        if !set {
            msg.push_str(&format!(
                "\n\nWARNING: {ek} is not set in this shell.\n\
                 While logged into xAI, an unset env_key falls through to your session token,\n\
                 and third-party APIs return 401 (looks like: Auth recovery succeeded but still 401).\n\
                 Fix: export {ek}=… then restart dgrok."
            ));
        }
    }
    Ok(msg)
}

pub fn format_presets() -> String {
    let mut out = String::from(
        "Presets write official [model.*] blocks into ~/.grok/config.toml\n\
         (same format as 11-custom-models.md — not a separate provider stack).\n\n",
    );
    for p in builtin_presets() {
        out.push_str(&format!(
            "  {id:<12} {name}\n    base_url={base}\n    model={model}  env_key={env}\n    {notes}\n\n",
            id = p.id,
            name = p.name,
            base = p.base_url,
            model = p.model,
            env = p.env_key.unwrap_or("(none)"),
            notes = p.notes,
        ));
    }
    out.push_str(
        "Examples:\n         dgrok provider add nvidia --model z-ai/glm-5.2 --set-default\n         export NVIDIA_API_KEY=…\n         dgrok provider add opencode --set-default\n         export OPENCODE_API_KEY=…   # from https://opencode.ai/zen\n         dgrok -m opencode\n         \n         Pitfalls that look like 401 auth recovery:\n         - base_url must NOT end with /chat/completions (Grok adds that path)\n         - OpenCode API model id is bare (mimo-v2.5-free), not opencode/mimo-v2.5-free\n         - env_key must be exported in the same shell before launching dgrok\n",
    );
    out
}

pub fn format_list(path: &Path) -> String {
    let Some(doc) = read_config_document_for_edit(path) else {
        return format!(
            "Could not parse {} — fix TOML syntax before listing models.",
            path.display()
        );
    };
    let ids = list_model_ids(&doc);
    let default = doc
        .get("models")
        .and_then(|m| m.get("default"))
        .and_then(|v| v.as_str())
        .unwrap_or("(unset)");
    if ids.is_empty() {
        return format!(
            "No [model.*] entries in {}\n[models].default = {default}\n\nAdd one: dgrok provider presets\nDocs: {CUSTOM_MODELS_DOC}",
            path.display()
        );
    }
    let mut out = format!(
        "Custom models in {}  ([models].default = {default})\n",
        path.display()
    );
    for id in ids {
        let entry = doc
            .get("model")
            .and_then(|m| m.get(id.as_str()))
            .and_then(|t| t.as_table());
        let model = entry
            .and_then(|t| t.get("model"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let base = entry
            .and_then(|t| t.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        out.push_str(&format!("  {id:<16} model={model}  base_url={base}\n"));
    }
    out.push_str("\nSwitch: dgrok -m <id>  |  /model <id>\n");
    out
}

/// Doctor: report config path, models, env keys present (not secret values).
pub fn run_doctor() -> String {
    let path = user_config_toml_path();
    let mut lines = Vec::new();
    lines.push("dgrok doctor — official multi-provider config check".to_string());
    lines.push(format!("config: {}", path.display()));
    lines.push(format!(
        "exists: {}",
        if path.is_file() { "yes" } else { "no (will create on first write)" }
    ));

    if path.is_file() {
        match std::fs::read_to_string(&path) {
            Ok(s) => match s.parse::<toml_edit::DocumentMut>() {
                Ok(doc) => {
                    let ids = list_model_ids(&doc);
                    lines.push(format!("[model.*] count: {}", ids.len()));
                    let default = doc
                        .get("models")
                        .and_then(|m| m.get("default"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("(unset)");
                    lines.push(format!("[models].default: {default}"));
                    for id in &ids {
                        if let Some(table) = doc
                            .get("model")
                            .and_then(|m| m.get(id.as_str()))
                            .and_then(|t| t.as_table())
                        {
                            let ek = table.get("env_key").and_then(|v| v.as_str());
                            let has_api_key = table.get("api_key").is_some();
                            let model = table.get("model").and_then(|v| v.as_str()).unwrap_or("?");
                            let base = table.get("base_url").and_then(|v| v.as_str()).unwrap_or("?");
                            let env_status = match ek {
                                Some(name) => {
                                    if std::env::var(name).map(|v| !v.trim().is_empty()).unwrap_or(false)
                                    {
                                        format!("env_key={name} set=yes")
                                    } else {
                                        format!(
                                            "env_key={name} set=no  ← export {name}=… or third-party 401"
                                        )
                                    }
                                }
                                None if has_api_key => "api_key=present (prefer env_key)".into(),
                                None => "no env_key/api_key (session token will be sent)".into(),
                            };
                            lines.push(format!("  {id}: model={model}"));
                            lines.push(format!("       base_url={base}"));
                            lines.push(format!("       {env_status}"));
                            if base.contains("/chat/completions")
                                || base.contains("/messages")
                                || base.ends_with("/responses")
                            {
                                lines.push(
                                    "       ⚠ base_url includes an API path suffix — use the /v1 root only"
                                        .into(),
                                );
                            }
                            if model.starts_with("opencode/") || model.starts_with("opencode-go/") {
                                lines.push(
                                    "       ⚠ model id has opencode/ prefix — Zen HTTP API wants bare id"
                                        .into(),
                                );
                            }
                        }
                    }
                    if ids.is_empty() {
                        lines.push(
                            "No custom models — `dgrok provider add …` never wrote [model.*], or config was reset."
                                .into(),
                        );
                    }
                }
                Err(e) => lines.push(format!("TOML parse error: {e}")),
            },
            Err(e) => lines.push(format!("read error: {e}")),
        }
    }

    lines.push(String::new());
    lines.push("401 \"Auth recovery succeeded but … still rejected\" usually means:".into());
    lines.push("  1) env_key not exported → Grok sends your xAI session token to a foreign host".into());
    lines.push("  2) wrong API model id (e.g. opencode/mimo-v2.5-free → bare mimo-v2.5-free)".into());
    lines.push("  3) base_url ends with /chat/completions (double path) or invalid API key".into());
    lines.push(String::new());
    lines.push("Official multi-provider docs:".into());
    lines.push(format!("  {CUSTOM_MODELS_DOC}"));
    lines.push("Quick setup (OpenClaude-style wizard):".into());
    lines.push("  dgrok provider setup".into());
    lines.push("  # or in TUI: /provider setup".into());
    lines.push("  dgrok provider endpoints   # list NVIDIA, OpenCode, Anthropic, Codex, …".into());
    lines.push("  dgrok -m <config-id>       # after setup; /model shows provider on the right".into());
    lines.join("\n")
}

pub fn run_provider_cli(args: ProviderArgs) -> Result<(), String> {
    let path = user_config_toml_path();
    match args.command.unwrap_or(ProviderCommand::Guide) {
        ProviderCommand::Guide => {
            println!(
                "dgrok provider setup — pick an endpoint, paste a key, choose models. Start here.\n\
                 (Same wizard as TUI /provider, no args.)\n\n\
                 One-off fixes, not everyday use:\n\
                   set-key              paste a key onto models that 401 (env var was unset)\n\
                   list                 already-added models in config.toml\n\
                   endpoints            the wizard's endpoint list, as text\n\
                   add <id|preset>      non-interactive add, for scripting (--model --base-url --env-key --api-backend --set-default --force)\n\
                   presets              raw preset table `add` reads from\n\n\
                 Docs: {CUSTOM_MODELS_DOC}\n\
                 Diagnose 401s: dgrok doctor"
            );
            Ok(())
        }
        ProviderCommand::List => {
            print!("{}", format_list(&path));
            Ok(())
        }
        ProviderCommand::Presets => {
            print!("{}", format_presets());
            Ok(())
        }
        ProviderCommand::Endpoints => {
            print!("{}", crate::provider_setup::format_endpoint_catalog());
            Ok(())
        }
        ProviderCommand::Setup => {
            let msg = crate::provider_setup::run_interactive_setup()?;
            println!("{msg}");
            Ok(())
        }
        ProviderCommand::SetKey {
            env_key,
            base_url_contains,
            key,
        } => {
            let path = user_config_toml_path();
            let msg = if let Some(k) = key {
                crate::provider_setup::set_api_key_on_matching_models(
                    &path,
                    env_key.as_deref().or(Some("OPENCODE_API_KEY")),
                    base_url_contains.as_deref(),
                    &k,
                )?
            } else {
                crate::provider_setup::run_interactive_set_key(
                    env_key.as_deref(),
                    base_url_contains.as_deref(),
                )?
            };
            println!("{msg}");
            Ok(())
        }
        ProviderCommand::Add {
            id,
            base_url,
            model,
            env_key,
            api_backend,
            set_default,
            force,
        } => {
            let preset = builtin_presets().iter().find(|p| p.id == id);
            let (mid, mmodel, mbase, mname, menv, mbackend) = if let Some(p) = preset {
                (
                    p.id.to_string(),
                    model.unwrap_or_else(|| p.model.to_string()),
                    base_url.unwrap_or_else(|| p.base_url.to_string()),
                    Some(p.name.to_string()),
                    env_key.or_else(|| p.env_key.map(str::to_string)),
                    api_backend.or_else(|| p.api_backend.map(str::to_string)),
                )
            } else {
                let m = model.ok_or_else(|| {
                    "custom id requires --model (or use a preset: nvidia|opencode|openai|anthropic|ollama|openrouter)"
                        .to_string()
                })?;
                let b = base_url.ok_or_else(|| {
                    "custom id requires --base-url (or use a preset name as id)".to_string()
                })?;
                (id, m, b, None, env_key, api_backend)
            };
            let msg = upsert_model_table(
                &path,
                &mid,
                &mmodel,
                &mbase,
                mname.as_deref(),
                menv.as_deref(),
                mbackend.as_deref(),
                set_default,
                force,
            )?;
            println!("{msg}");
            Ok(())
        }
    }
}

/// Parse `/provider …` slash args into the same CLI surface.
pub fn run_provider_slash(args: &str) -> String {
    let trimmed = args.trim();
    if trimmed.is_empty() || trimmed == "help" || trimmed == "guide" {
        return "Add a provider: run /provider (no args) — that's the wizard, use it.\n\
                \n\
                Everything below is for one-off fixes, not everyday use:\n\
                /provider list                already-added models\n\
                /provider set-key            paste a key onto models that 401 (unset env var)\n\
                /provider endpoints          the wizard's endpoint list, as text\n\
                /provider add <id> …         non-interactive add (scripting)\n\
                /provider presets            raw preset table `add` reads from\n\
                Docs: 11-custom-models.md"
            .into();
    }
    // "setup" never reaches here — slash/commands/provider.rs::run intercepts
    // empty/"setup" and returns Action::OpenProviderSetup (TUI wizard) first.
    if trimmed == "endpoints" {
        return crate::provider_setup::format_endpoint_catalog();
    }
    // Reuse clap by synthesizing argv
    let mut argv = vec!["provider".to_string()];
    for part in trimmed.split_whitespace() {
        argv.push(part.to_string());
    }
    match ProviderArgs::try_parse_from(&argv) {
        Ok(a) => {
            let path = user_config_toml_path();
            match a.command.unwrap_or(ProviderCommand::Guide) {
                ProviderCommand::Guide => run_provider_slash(""),
                ProviderCommand::List => format_list(&path),
                ProviderCommand::Presets => format_presets(),
                ProviderCommand::Endpoints => crate::provider_setup::format_endpoint_catalog(),
                ProviderCommand::Setup => {
                    "Use TUI wizard (Action) or: dgrok provider setup".into()
                }
                ProviderCommand::SetKey { .. } => {
                    "Run in a terminal: dgrok provider set-key\n\
                     Example: dgrok provider set-key --env-key OPENCODE_API_KEY\n\
                     Example: dgrok provider set-key --base-url-contains nvidia.com"
                        .into()
                }
                ProviderCommand::Add {
                    id,
                    base_url,
                    model,
                    env_key,
                    api_backend,
                    set_default,
                    force,
                } => {
                    let preset = builtin_presets().iter().find(|p| p.id == id);
                    let result = if let Some(p) = preset {
                        upsert_model_table(
                            &path,
                            p.id,
                            model.as_deref().unwrap_or(p.model),
                            base_url.as_deref().unwrap_or(p.base_url),
                            Some(p.name),
                            env_key.as_deref().or(p.env_key),
                            api_backend.as_deref().or(p.api_backend),
                            set_default,
                            force,
                        )
                    } else if let (Some(m), Some(b)) = (model.as_deref(), base_url.as_deref()) {
                        upsert_model_table(
                            &path,
                            &id,
                            m,
                            b,
                            None,
                            env_key.as_deref(),
                            api_backend.as_deref(),
                            set_default,
                            force,
                        )
                    } else {
                        Err(
                            "add requires a preset name, or --model and --base-url for a custom id"
                                .into(),
                        )
                    };
                    match result {
                        Ok(s) => s,
                        Err(e) => format!("Error: {e}"),
                    }
                }
            }
        }
        Err(e) => format!("Usage: /provider list|presets|add …\n{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn upsert_writes_official_model_table() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[ui]\ntheme = \"dark\"\n").unwrap();
        upsert_model_table(
            &path,
            "nvidia",
            "z-ai/glm-5.2",
            "https://integrate.api.nvidia.com/v1",
            Some("NVIDIA NIM"),
            Some("NVIDIA_API_KEY"),
            Some("chat_completions"),
            true,
            false,
        )
        .unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("[model.nvidia]") || body.contains("[model.nvidia]"));
        assert!(body.contains("z-ai/glm-5.2"));
        assert!(body.contains("NVIDIA_API_KEY"));
        assert!(body.contains("default") && body.contains("nvidia"));
        assert!(body.contains("theme"));
    }

    #[test]
    fn normalize_strips_chat_completions_and_opencode_prefix() {
        assert_eq!(
            normalize_provider_base_url("https://opencode.ai/zen/v1/chat/completions"),
            "https://opencode.ai/zen/v1"
        );
        assert_eq!(
            normalize_provider_model_id("opencode/mimo-v2.5-free"),
            "mimo-v2.5-free"
        );
    }

    #[test]
    fn upsert_opencode_sanitizes_pasted_docs_values() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        upsert_model_table(
            &path,
            "opencode",
            "opencode/mimo-v2.5-free",
            "https://opencode.ai/zen/v1/chat/completions",
            Some("OpenCode Zen"),
            Some("OPENCODE_API_KEY"),
            Some("chat_completions"),
            true,
            false,
        )
        .unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("mimo-v2.5-free"));
        assert!(!body.contains("opencode/mimo"));
        assert!(body.contains("https://opencode.ai/zen/v1"));
        assert!(!body.contains("/chat/completions"));
    }

    #[test]
    fn refuses_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        upsert_model_table(
            &path,
            "x",
            "m",
            "http://localhost/v1",
            None,
            None,
            None,
            false,
            false,
        )
        .unwrap();
        let err = upsert_model_table(
            &path,
            "x",
            "m2",
            "http://localhost/v1",
            None,
            None,
            None,
            false,
            false,
        )
        .unwrap_err();
        assert!(err.contains("--force"));
    }

    #[test]
    fn list_ids_from_doc() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        upsert_model_table(
            &path,
            "a",
            "m",
            "http://x/v1",
            None,
            None,
            None,
            false,
            false,
        )
        .unwrap();
        let doc = read_config_document_for_edit(&path).unwrap();
        assert_eq!(list_model_ids(&doc), vec!["a".to_string()]);
    }
}
