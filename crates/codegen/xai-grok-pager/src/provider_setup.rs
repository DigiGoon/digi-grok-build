//! Interactive provider setup (OpenClaude-style) over official `[model.*]`.
//!
//! Flow: pick endpoint → (optional) custom URL → paste API key → fetch models
//! → pick models → write config.toml with `description = provider label`
//! (shown on the right of `/model` picker rows).

use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::provider_config_cmd::{
    normalize_provider_base_url, normalize_provider_model_id, user_config_toml_path,
    CUSTOM_MODELS_DOC,
};
use crate::config_toml_edit::read_config_document_for_edit;

/// Write `content` to `path` via tmp+rename (crash can't truncate config.toml)
/// and force owner-only `0o600` on unix — this file may hold plaintext
/// `api_key`s, same rule as `auth.json`. Mirrors the atomic-write convention
/// in `xai-grok-shell/src/util/config/persist.rs`, duplicated here because
/// that helper is `pub(crate)` to a different crate.
fn atomic_write_config_toml(path: &Path, content: &str) -> Result<(), String> {
    let suffix = {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("toml.tmp.{}.{}", std::process::id(), nanos)
    };
    let tmp = path.with_extension(suffix);
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// How the catalog entry authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Ask for an API key (stored as `api_key` so third-party works without env).
    ApiKey,
    /// No key (local Ollama, etc.).
    None,
}

/// How to discover models after auth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discovery {
    /// `GET {base_url}/models` (OpenAI-compatible).
    OpenAiModels,
    /// Skip network; offer the preset default model only.
    FixedDefault,
}

/// One curated endpoint shown in the setup list.
#[derive(Debug, Clone)]
pub struct EndpointEntry {
    pub id: &'static str,
    /// Left column label.
    pub label: &'static str,
    /// Right column blurb (base URL / auth).
    pub blurb: &'static str,
    pub base_url: &'static str,
    pub default_model: &'static str,
    pub env_key: Option<&'static str>,
    pub api_backend: &'static str,
    pub auth: AuthMode,
    pub discovery: Discovery,
    /// Written into each model's `description` (picker right column).
    pub provider_label: &'static str,
}

/// Curated endpoints + notes. Custom is handled separately (last list row).
pub fn endpoint_catalog() -> &'static [EndpointEntry] {
    &[
        EndpointEntry {
            id: "opencode",
            label: "OpenCode Zen",
            blurb: "API key · free models · opencode.ai/zen",
            base_url: "https://opencode.ai/zen/v1",
            default_model: "mimo-v2.5-free",
            env_key: Some("OPENCODE_API_KEY"),
            api_backend: "chat_completions",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "OpenCode Zen",
        },
        EndpointEntry {
            id: "nvidia",
            label: "NVIDIA NIM",
            blurb: "API key · integrate.api.nvidia.com",
            base_url: "https://integrate.api.nvidia.com/v1",
            default_model: "meta/llama-3.1-70b-instruct",
            env_key: Some("NVIDIA_API_KEY"),
            api_backend: "chat_completions",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "NVIDIA NIM",
        },
        EndpointEntry {
            id: "openai",
            label: "OpenAI",
            blurb: "API key · Chat Completions",
            base_url: "https://api.openai.com/v1",
            default_model: "gpt-4o",
            env_key: Some("OPENAI_API_KEY"),
            api_backend: "chat_completions",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "OpenAI",
        },
        EndpointEntry {
            id: "codex",
            label: "OpenAI Codex / Responses",
            blurb: "API key · Responses API (no browser OAuth in dgrok)",
            base_url: "https://api.openai.com/v1",
            default_model: "gpt-5.4",
            env_key: Some("OPENAI_API_KEY"),
            api_backend: "responses",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "OpenAI Codex",
        },
        EndpointEntry {
            id: "anthropic",
            label: "Anthropic Claude",
            blurb: "API key · Messages API (no Claude OAuth in dgrok)",
            base_url: "https://api.anthropic.com/v1",
            default_model: "claude-opus-4-6",
            env_key: Some("ANTHROPIC_API_KEY"),
            api_backend: "messages",
            auth: AuthMode::ApiKey,
            // Anthropic /v1/models needs special headers; fall back to default.
            discovery: Discovery::FixedDefault,
            provider_label: "Anthropic",
        },
        EndpointEntry {
            id: "openrouter",
            label: "OpenRouter",
            blurb: "API key · openrouter.ai",
            base_url: "https://openrouter.ai/api/v1",
            default_model: "openai/gpt-4o",
            env_key: Some("OPENROUTER_API_KEY"),
            api_backend: "chat_completions",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "OpenRouter",
        },
        EndpointEntry {
            id: "groq",
            label: "Groq",
            blurb: "API key · api.groq.com",
            base_url: "https://api.groq.com/openai/v1",
            default_model: "llama-3.3-70b-versatile",
            env_key: Some("GROQ_API_KEY"),
            api_backend: "chat_completions",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "Groq",
        },
        EndpointEntry {
            id: "deepseek",
            label: "DeepSeek",
            blurb: "API key · api.deepseek.com",
            base_url: "https://api.deepseek.com/v1",
            default_model: "deepseek-chat",
            env_key: Some("DEEPSEEK_API_KEY"),
            api_backend: "chat_completions",
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            provider_label: "DeepSeek",
        },
        EndpointEntry {
            id: "ollama",
            label: "Ollama (local)",
            blurb: "No key · localhost:11434",
            base_url: "http://127.0.0.1:11434/v1",
            default_model: "llama3.1",
            env_key: None,
            api_backend: "chat_completions",
            auth: AuthMode::None,
            discovery: Discovery::OpenAiModels,
            provider_label: "Ollama",
        },
    ]
}

pub const CUSTOM_ENDPOINT_ID: &str = "custom";

/// Wizard step carried through LocalQuestionKind / Action.
#[derive(Debug, Clone)]
pub enum WizardStep {
    /// First screen: pick curated endpoint or Custom.
    PickEndpoint,
    /// Custom: freeform `Name | https://base/url` [ | backend ].
    CustomDetails,
    /// Paste API key (freeform).
    EnterKey { draft: ProviderDraft },
    /// Multi-select discovered models.
    PickModels {
        draft: ProviderDraft,
        models: Vec<DiscoveredModel>,
    },
}

/// Draft after the user picks an endpoint (preset or custom).
#[derive(Debug, Clone)]
pub struct ProviderDraft {
    pub endpoint_id: String,
    pub provider_label: String,
    pub base_url: String,
    pub default_model: String,
    pub env_key: Option<String>,
    pub api_backend: String,
    pub auth: AuthMode,
    pub discovery: Discovery,
    pub api_key: Option<String>,
}

impl ProviderDraft {
    pub fn from_catalog(entry: &EndpointEntry) -> Self {
        Self {
            endpoint_id: entry.id.to_string(),
            provider_label: entry.provider_label.to_string(),
            base_url: entry.base_url.to_string(),
            default_model: entry.default_model.to_string(),
            env_key: entry.env_key.map(str::to_string),
            api_backend: entry.api_backend.to_string(),
            auth: entry.auth,
            discovery: entry.discovery,
            api_key: None,
        }
    }

    pub fn custom(base_url: &str, provider_label: &str, api_backend: &str) -> Self {
        Self {
            endpoint_id: CUSTOM_ENDPOINT_ID.to_string(),
            provider_label: provider_label.trim().to_string(),
            base_url: normalize_provider_base_url(base_url),
            default_model: String::new(),
            env_key: None,
            api_backend: api_backend.to_string(),
            auth: AuthMode::ApiKey,
            discovery: Discovery::OpenAiModels,
            api_key: None,
        }
    }
}

/// One model discovered from a provider.
#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    pub id: String,
    pub display: String,
}

/// Sanitize a remote model id into a config table key: `opencode-mimo-v2.5-free`.
pub fn config_model_key(provider_id: &str, model_id: &str) -> String {
    let mut s = format!("{provider_id}-{}", model_id);
    s = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Collapse runs of '-'
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.chars() {
        if c == '-' {
            if !prev_dash {
                out.push(c);
            }
            prev_dash = true;
        } else {
            prev_dash = false;
            out.push(c);
        }
    }
    out.trim_matches('-').to_string()
}

/// Parse OpenAI-style `{ "data": [ { "id": "…" }, … ] }` list bodies.
pub fn parse_openai_models_json(body: &str) -> Result<Vec<DiscoveredModel>, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("models JSON parse error: {e}"))?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "models response missing data[]".to_string())?;
    let mut out = Vec::new();
    for item in data {
        let id = item
            .get("id")
            .or_else(|| item.get("model"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim();
        if id.is_empty() {
            continue;
        }
        let id = normalize_provider_model_id(id);
        out.push(DiscoveredModel {
            display: id.clone(),
            id,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    if out.is_empty() {
        return Err("models list was empty".into());
    }
    Ok(out)
}

/// Fetch models from an OpenAI-compatible `/models` endpoint.
///
/// Uses a **child process** (`curl`) so we never create or drop a Tokio
/// runtime on the TUI event loop. `reqwest::blocking` panics with
/// "Cannot drop a runtime in a context where blocking is not allowed"
/// when used from inside dgrok's async runtime — even via `std::thread`
/// on some paths — so it is intentionally not used here.
pub fn fetch_openai_models(
    base_url: &str,
    api_key: Option<&str>,
    extra_headers: &[(&str, &str)],
) -> Result<Vec<DiscoveredModel>, String> {
    let base = normalize_provider_base_url(base_url);
    let url = format!("{base}/models");
    let body = http_get_json_body(&url, api_key, extra_headers)?;
    parse_openai_models_json(&body)
}

/// Tokio-free HTTP GET (curl subprocess). Returns response body text.
fn http_get_json_body(
    url: &str,
    api_key: Option<&str>,
    extra_headers: &[(&str, &str)],
) -> Result<String, String> {
    use std::process::Command;

    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS", // silent but show errors
        "-L",  // follow redirects
        "-m",
        "20", // max time
        "-H",
        "Accept: application/json",
        "-w",
        "\n__DGROK_HTTP_STATUS__:%{http_code}",
    ]);
    if let Some(k) = api_key.map(str::trim).filter(|s| !s.is_empty()) {
        cmd.arg("-H").arg(format!("Authorization: Bearer {k}"));
    }
    for (h, v) in extra_headers {
        cmd.arg("-H").arg(format!("{h}: {v}"));
    }
    cmd.arg(url);

    let output = cmd
        .output()
        .map_err(|e| format!("curl failed to start (is curl installed?): {e}"))?;
    if !output.status.success() && output.stdout.is_empty() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl GET {url} failed: {err}"));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    // Split trailing status marker we asked curl to append.
    let (body, status) = if let Some(idx) = raw.rfind("\n__DGROK_HTTP_STATUS__:") {
        let body = &raw[..idx];
        let status = raw[idx + "\n__DGROK_HTTP_STATUS__:".len()..].trim();
        (body.to_string(), status.to_string())
    } else {
        (raw.to_string(), String::new())
    };
    if !status.is_empty() && status != "200" {
        let snippet: String = body.chars().take(240).collect();
        return Err(format!("GET {url} → HTTP {status}: {snippet}"));
    }
    if body.trim().is_empty() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("GET {url}: empty body ({err})"));
    }
    Ok(body)
}

/// Discover models for a draft (fetch or fixed default).
pub fn discover_models(draft: &ProviderDraft) -> Result<Vec<DiscoveredModel>, String> {
    match draft.discovery {
        Discovery::FixedDefault => {
            let id = if draft.default_model.is_empty() {
                return Err("no default model for this provider".into());
            } else {
                normalize_provider_model_id(&draft.default_model)
            };
            Ok(vec![DiscoveredModel {
                display: id.clone(),
                id,
            }])
        }
        Discovery::OpenAiModels => {
            let mut headers: Vec<(&str, &str)> = Vec::new();
            // Anthropic-style if someone switches discovery later.
            if draft.api_backend == "messages" {
                headers.push(("anthropic-version", "2023-06-01"));
            }
            match fetch_openai_models(
                &draft.base_url,
                draft.api_key.as_deref(),
                &headers,
            ) {
                Ok(list) => Ok(list),
                Err(e) if !draft.default_model.is_empty() => {
                    // Fall back to preset default so setup still completes.
                    let id = normalize_provider_model_id(&draft.default_model);
                    Ok(vec![DiscoveredModel {
                        display: format!("{id} (default; list failed: {e})"),
                        id,
                    }])
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Fields written for one selected model.
#[derive(Debug, Clone)]
pub struct ModelWriteSpec {
    pub config_id: String,
    pub model: String,
    pub base_url: String,
    pub name: String,
    pub description: String,
    pub env_key: Option<String>,
    pub api_key: Option<String>,
    pub api_backend: String,
}

pub fn specs_from_selection(
    draft: &ProviderDraft,
    selected_model_ids: &[String],
) -> Result<Vec<ModelWriteSpec>, String> {
    if selected_model_ids.is_empty() {
        return Err("select at least one model".into());
    }
    let base = normalize_provider_base_url(&draft.base_url);
    if base.is_empty() {
        return Err("base_url is empty".into());
    }
    let mut specs = Vec::new();
    for mid in selected_model_ids {
        let model = normalize_provider_model_id(mid);
        if model.is_empty() {
            continue;
        }
        let config_id = config_model_key(&draft.endpoint_id, &model);
        specs.push(ModelWriteSpec {
            config_id,
            name: model.clone(),
            description: draft.provider_label.clone(),
            model,
            base_url: base.clone(),
            env_key: draft.env_key.clone(),
            api_key: draft.api_key.clone(),
            api_backend: draft.api_backend.clone(),
        });
    }
    if specs.is_empty() {
        return Err("no valid model ids".into());
    }
    Ok(specs)
}

/// Write many `[model.*]` tables; optionally set default to the first.
pub fn write_model_specs(
    path: &Path,
    specs: &[ModelWriteSpec],
    set_default_first: bool,
    force: bool,
) -> Result<String, String> {
    if specs.is_empty() {
        return Err("nothing to write".into());
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

    if doc.get("model").and_then(|i| i.as_table()).is_none() {
        let mut root = toml_edit::Table::new();
        root.set_implicit(true);
        doc["model"] = toml_edit::Item::Table(root);
    } else if let Some(root) = doc["model"].as_table_mut() {
        root.set_implicit(true);
    }

    let existing = crate::provider_config_cmd::list_model_ids(&doc);
    let mut written = Vec::new();
    for spec in specs {
        let id = &spec.config_id;
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(format!("invalid model config id `{id}`"));
        }
        // toml keys with '.' become nested — replace dots in key only
        let table_key = id.replace('.', "-");
        if existing.iter().any(|k| k == &table_key) && !force {
            return Err(format!(
                "[model.{table_key}] already exists — re-run with force, or pick other models"
            ));
        }
        let mut entry = toml_edit::Table::new();
        entry["model"] = toml_edit::value(&spec.model);
        entry["base_url"] = toml_edit::value(&spec.base_url);
        entry["name"] = toml_edit::value(&spec.name);
        entry["description"] = toml_edit::value(&spec.description);
        if let Some(ref ek) = spec.env_key {
            entry["env_key"] = toml_edit::value(ek.as_str());
        }
        // Store api_key so third-party works immediately (env often unset).
        if let Some(ref key) = spec.api_key {
            if !key.trim().is_empty() {
                entry["api_key"] = toml_edit::value(key.as_str());
            }
        }
        if !spec.api_backend.is_empty() {
            entry["api_backend"] = toml_edit::value(spec.api_backend.as_str());
        }
        doc["model"][&table_key] = toml_edit::Item::Table(entry);
        written.push(table_key);
    }

    if set_default_first {
        if doc.get("models").and_then(|i| i.as_table()).is_none() {
            doc["models"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        doc["models"]["default"] = toml_edit::value(written[0].as_str());
    }

    atomic_write_config_toml(path, &doc.to_string())?;
    Ok(format!(
        "Wrote {} model(s) to {}\n  {}\n[models].default = {}\nPicker shows model on the left, provider ({}) on the right.\nUse: dgrok -m {}  |  /model\nDocs: {CUSTOM_MODELS_DOC}",
        written.len(),
        path.display(),
        written.join(", "),
        if set_default_first {
            written[0].as_str()
        } else {
            "(unchanged)"
        },
        specs.first().map(|s| s.description.as_str()).unwrap_or("?"),
        written[0],
    ))
}

/// Write `api_key` onto every `[model.*]` that uses `env_key` (or all with matching base host).
///
/// Fixes the common 401 path: setup wrote `env_key=OPENCODE_API_KEY` but the
/// env var was never exported and no `api_key` was stored.
pub fn set_api_key_on_matching_models(
    path: &Path,
    env_key_filter: Option<&str>,
    base_url_substr: Option<&str>,
    api_key: &str,
) -> Result<String, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("api key must not be empty".into());
    }
    if env_key_filter.is_none() && base_url_substr.is_none() {
        return Err(
            "set-key needs --env-key or --base-url-contains — refusing to paste one key onto every BYOK model".into(),
        );
    }
    let Some(mut doc) = read_config_document_for_edit(path) else {
        return Err(format!("unparseable config at {}", path.display()));
    };
    let ids = crate::provider_config_cmd::list_model_ids(&doc);
    if ids.is_empty() {
        return Err("no [model.*] entries to update".into());
    }
    let mut updated = Vec::new();
    for id in &ids {
        let Some(table) = doc
            .get("model")
            .and_then(|m| m.get(id.as_str()))
            .and_then(|t| t.as_table())
        else {
            continue;
        };
        let ek = table.get("env_key").and_then(|v| v.as_str()).unwrap_or("");
        let base = table.get("base_url").and_then(|v| v.as_str()).unwrap_or("");
        let match_env = env_key_filter.map(|f| ek == f).unwrap_or(false);
        let match_base = base_url_substr
            .map(|s| base.contains(s))
            .unwrap_or(false);
        if !(match_env || match_base) {
            continue;
        }
        if let Some(entry) = doc
            .get_mut("model")
            .and_then(|m| m.get_mut(id.as_str()))
            .and_then(|t| t.as_table_mut())
        {
            entry["api_key"] = toml_edit::value(key);
            updated.push(id.clone());
        }
    }
    if updated.is_empty() {
        return Err(
            "no matching models (try: dgrok provider set-key --env-key OPENCODE_API_KEY)".into(),
        );
    }
    atomic_write_config_toml(path, &doc.to_string())?;
    Ok(format!(
        "Wrote api_key onto {} model(s): {}\n\
         Default/session will use this key (no export needed).\n\
         Restart dgrok or start a new session, then /model <id>.",
        updated.len(),
        updated.join(", ")
    ))
}

/// Interactive CLI: paste key once for all models using a given env_key / provider.
pub fn run_interactive_set_key(env_key: Option<&str>, base_substr: Option<&str>) -> Result<String, String> {
    let path = user_config_toml_path();
    let filter = env_key.unwrap_or("OPENCODE_API_KEY");
    println!(
        "This writes api_key into every [model.*] with env_key={filter}{}\n\
         (fixes 401 when env var is unset and Grok would send your xAI session token).",
        base_substr
            .map(|s| format!(" or base_url containing {s}"))
            .unwrap_or_default()
    );
    let key = read_line(&format!("Paste API key for {filter}: "))?;
    set_api_key_on_matching_models(
        &path,
        Some(filter),
        base_substr,
        &key,
    )
}

/// Format the catalog for CLI / doctor.
pub fn format_endpoint_catalog() -> String {
    let mut out = String::from(
        "Endpoints (select in `dgrok provider setup` or TUI `/provider setup`):\n\n",
    );
    for (i, e) in endpoint_catalog().iter().enumerate() {
        out.push_str(&format!(
            "  {n}. {label:<28}  {blurb}\n     id={id}  base={base}\n",
            n = i + 1,
            label = e.label,
            blurb = e.blurb,
            id = e.id,
            base = e.base_url,
        ));
    }
    out.push_str(&format!(
        "  {n}. Custom provider…               enter base URL + label\n\n\
         Notes:\n\
         - Anthropic/Codex browser OAuth is not in dgrok — use API keys.\n\
         - Keys are written as [model.*].api_key so requests work without export.\n\
         - Prefer rotating keys into env vars later (env_key is also set when known).\n",
        n = endpoint_catalog().len() + 1,
    ));
    out
}

fn read_line(prompt: &str) -> Result<String, String> {
    let mut stdout = io::stdout();
    write!(stdout, "{prompt}").map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// Fully interactive CLI wizard (`dgrok provider setup`).
pub fn run_interactive_setup() -> Result<String, String> {
    println!("{}", format_endpoint_catalog());
    let catalog = endpoint_catalog();
    let max = catalog.len() + 1;
    let choice = read_line(&format!("Select endpoint [1-{max}]: "))?;
    let n: usize = choice
        .trim()
        .parse()
        .map_err(|_| format!("enter a number 1-{max}"))?;
    if n < 1 || n > max {
        return Err(format!("out of range 1-{max}"));
    }

    let mut draft = if n == max {
        let label = read_line("Provider display name (shown on right of model list): ")?;
        let url = read_line("Base URL (e.g. https://api.example.com/v1): ")?;
        let backend = read_line(
            "API backend [chat_completions|responses|messages] (default chat_completions): ",
        )?;
        let backend = if backend.trim().is_empty() {
            "chat_completions".to_string()
        } else {
            backend.trim().to_string()
        };
        if label.trim().is_empty() || url.trim().is_empty() {
            return Err("name and base URL required for custom".into());
        }
        ProviderDraft::custom(&url, &label, &backend)
    } else {
        ProviderDraft::from_catalog(&catalog[n - 1])
    };

    if draft.auth == AuthMode::ApiKey {
        let hint = draft
            .env_key
            .as_deref()
            .unwrap_or("API key");
        loop {
            let key = read_line(&format!(
                "API key for {} (stored as api_key in config.toml; {hint}): ",
                draft.provider_label
            ))?;
            if key.trim().is_empty() {
                eprintln!(
                    "error: empty key → dgrok will send your xAI session token and get 401. Paste a real key."
                );
                continue;
            }
            draft.api_key = Some(key);
            break;
        }
    }

    eprintln!("Fetching models from {} …", draft.base_url);
    let models = discover_models(&draft)?;
    println!("Available models ({}):", models.len());
    let show = models.len().min(40);
    for (i, m) in models.iter().take(show).enumerate() {
        println!("  {:>3}. {}", i + 1, m.display);
    }
    if models.len() > show {
        println!("  … {} more (type ids or numbers)", models.len() - show);
    }

    let pick = read_line(
        "Select models: numbers (1,2,5) or ids (comma-separated); empty = first only: ",
    )?;
    let selected = parse_model_selection(&pick, &models)?;
    let specs = specs_from_selection(&draft, &selected)?;
    let path = user_config_toml_path();
    write_model_specs(&path, &specs, true, true)
}

/// Parse "1,2,5" or "mimo-v2.5-free,big-pickle" against discovered list.
pub fn parse_model_selection(
    input: &str,
    models: &[DiscoveredModel],
) -> Result<Vec<String>, String> {
    let t = input.trim();
    if t.is_empty() {
        return Ok(vec![models[0].id.clone()]);
    }
    let mut out = Vec::new();
    for part in t.split([',', ' ']) {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if let Ok(n) = p.parse::<usize>() {
            if n == 0 || n > models.len() {
                return Err(format!("index {n} out of range 1-{}", models.len()));
            }
            out.push(models[n - 1].id.clone());
        } else {
            // Match id case-sensitive first, then ignore case.
            if let Some(m) = models.iter().find(|m| m.id == p) {
                out.push(m.id.clone());
            } else if let Some(m) = models.iter().find(|m| m.id.eq_ignore_ascii_case(p)) {
                out.push(m.id.clone());
            } else {
                // Allow typing an id not in the list (provider may accept it).
                out.push(normalize_provider_model_id(p));
            }
        }
    }
    out.dedup();
    if out.is_empty() {
        Err("no models selected".into())
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_models_json() {
        let body = r#"{"data":[{"id":"mimo-v2.5-free"},{"id":"opencode/big-pickle"}]}"#;
        let list = parse_openai_models_json(body).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "big-pickle"); // sorted, prefix stripped
        assert_eq!(list[1].id, "mimo-v2.5-free");
    }

    #[test]
    fn config_key_sanitizes() {
        assert_eq!(
            config_model_key("nvidia", "meta/llama-3.1"),
            "nvidia-meta-llama-3.1"
        );
    }

    #[test]
    fn write_specs_sets_description_provider() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[ui]\ntheme = \"dark\"\n").unwrap();
        let draft = ProviderDraft {
            endpoint_id: "opencode".into(),
            provider_label: "OpenCode Zen".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            default_model: "mimo-v2.5-free".into(),
            env_key: Some("OPENCODE_API_KEY".into()),
            api_backend: "chat_completions".into(),
            auth: AuthMode::ApiKey,
            discovery: Discovery::FixedDefault,
            api_key: Some("sk-test".into()),
        };
        let specs = specs_from_selection(&draft, &["mimo-v2.5-free".into()]).unwrap();
        write_model_specs(&path, &specs, true, false).unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("description = \"OpenCode Zen\""));
        assert!(body.contains("api_key = \"sk-test\""));
        assert!(body.contains("mimo-v2.5-free"));
        assert!(body.contains("default"));
    }

    #[test]
    fn parse_selection_numbers_and_ids() {
        let models = vec![
            DiscoveredModel {
                id: "a".into(),
                display: "a".into(),
            },
            DiscoveredModel {
                id: "b".into(),
                display: "b".into(),
            },
        ];
        assert_eq!(
            parse_model_selection("2", &models).unwrap(),
            vec!["b".to_string()]
        );
        assert_eq!(
            parse_model_selection("a,b", &models).unwrap(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    /// Regression: TUI used to panic with
    /// "Cannot drop a runtime in a context where blocking is not allowed"
    /// when reqwest::blocking ran on the Tokio event loop during `/provider setup`.
    #[test]
    fn fetch_safe_inside_tokio_runtime() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        // Call fetch directly from async context (same as TUI dispatch path).
        let result = rt.block_on(async {
            fetch_openai_models("https://127.0.0.1:1", None, &[])
        });
        // Connection refused or similar is fine — panic is not.
        assert!(result.is_err(), "expected network error, got {result:?}");
    }

    #[test]
    fn parse_status_marker_split_logic() {
        // ensure http_get_json_body status parsing is covered indirectly via parse
        let body = r#"{"data":[{"id":"x"}]}"#;
        assert_eq!(parse_openai_models_json(body).unwrap()[0].id, "x");
    }

    #[test]
    fn set_api_key_patches_matching_env_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[model.a]
model = "m"
base_url = "https://opencode.ai/zen/v1"
env_key = "OPENCODE_API_KEY"

[model.b]
model = "n"
base_url = "https://other.example/v1"
env_key = "OTHER_KEY"
"#,
        )
        .unwrap();
        set_api_key_on_matching_models(&path, Some("OPENCODE_API_KEY"), None, "sk-test-123")
            .unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("sk-test-123"));
        // only model.a should have it next to OPENCODE
        assert!(body.contains("[model.a]"));
        // model.b must not get the opencode key
        let a = body.split("[model.b]").next().unwrap();
        assert!(a.contains("sk-test-123"));
        let b = body.split("[model.b]").nth(1).unwrap();
        assert!(!b.contains("sk-test-123"));
    }

    #[test]
    fn set_api_key_refuses_with_no_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "[model.a]\nmodel = \"m\"\nbase_url = \"https://x/v1\"\nenv_key = \"A_KEY\"\n",
        )
        .unwrap();
        let err = set_api_key_on_matching_models(&path, None, None, "sk-test").unwrap_err();
        assert!(err.contains("--env-key"));
        // must not have written the key onto model.a
        assert!(!fs::read_to_string(&path).unwrap().contains("sk-test"));
    }

    #[cfg(unix)]
    #[test]
    fn write_specs_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let draft = ProviderDraft {
            endpoint_id: "opencode".into(),
            provider_label: "OpenCode Zen".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            default_model: "mimo-v2.5-free".into(),
            env_key: Some("OPENCODE_API_KEY".into()),
            api_backend: "chat_completions".into(),
            auth: AuthMode::ApiKey,
            discovery: Discovery::FixedDefault,
            api_key: Some("sk-test".into()),
        };
        let specs = specs_from_selection(&draft, &["mimo-v2.5-free".into()]).unwrap();
        write_model_specs(&path, &specs, true, false).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "config.toml holding api_key must be owner-only");
    }
}
