//! `/provider` — UX over official multi-provider `config.toml` (not a parallel stack).

use crate::provider_config_cmd;
use crate::slash::command::{AppCtx, ArgItem, CommandExecCtx, CommandResult, SlashCommand};

pub struct ProviderCommand;

impl SlashCommand for ProviderCommand {
    fn name(&self) -> &str {
        "provider"
    }

    fn aliases(&self) -> &[&str] {
        &["providers", "init-model"]
    }

    fn description(&self) -> &str {
        "Manage custom models in config.toml (official multi-provider)"
    }

    fn usage(&self) -> &str {
        "/provider [list|presets|add …|help]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        false
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("[list|presets|add|help]")
    }

    fn suggest_args(&self, _ctx: &AppCtx, _args_query: &str) -> Option<Vec<ArgItem>> {
        Some(vec![
            ArgItem {
                display: "list".into(),
                match_text: "list".into(),
                insert_text: "list".into(),
                description: "List [model.*] in config.toml".into(),
            },
            ArgItem {
                display: "presets".into(),
                match_text: "presets".into(),
                insert_text: "presets".into(),
                description: "Show nvidia/openai/anthropic/ollama presets".into(),
            },
            ArgItem {
                display: "add nvidia".into(),
                match_text: "add nvidia".into(),
                insert_text: "add nvidia --set-default".into(),
                description: "Write NVIDIA NIM model block".into(),
            },
            ArgItem {
                display: "guide".into(),
                match_text: "guide".into(),
                insert_text: "guide".into(),
                description: "How this maps to official config".into(),
            },
        ])
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        CommandResult::Message(provider_config_cmd::run_provider_slash(args))
    }
}
