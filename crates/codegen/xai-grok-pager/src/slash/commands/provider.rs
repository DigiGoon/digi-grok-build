//! `/provider` — UX over official multi-provider `config.toml` (not a parallel stack).

use crate::app::actions::Action;
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
        "Add providers (OpenCode, NVIDIA, Anthropic, Codex, custom…) to config.toml"
    }

    fn usage(&self) -> &str {
        "/provider [setup|endpoints|list|presets|add …|help]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        false
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("[setup|endpoints|list|presets|add|help]")
    }

    fn suggest_args(&self, _ctx: &AppCtx, _args_query: &str) -> Option<Vec<ArgItem>> {
        Some(vec![
            ArgItem {
                display: "setup".into(),
                match_text: "setup".into(),
                insert_text: "setup".into(),
                description: "Interactive: endpoints → key → fetch models".into(),
            },
            ArgItem {
                display: "endpoints".into(),
                match_text: "endpoints".into(),
                insert_text: "endpoints".into(),
                description: "List NVIDIA, OpenCode, Anthropic, Codex, …".into(),
            },
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
                description: "Show writeable presets".into(),
            },
            ArgItem {
                display: "add nvidia".into(),
                match_text: "add nvidia".into(),
                insert_text: "add nvidia --set-default".into(),
                description: "Write NVIDIA NIM model block".into(),
            },
            ArgItem {
                display: "add opencode".into(),
                match_text: "add opencode".into(),
                insert_text: "add opencode --set-default".into(),
                description: "Write OpenCode Zen model block".into(),
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
        let trimmed = args.trim();
        // Empty or setup → interactive TUI wizard (OpenClaude-style).
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("setup") {
            return CommandResult::Action(Action::OpenProviderSetup);
        }
        CommandResult::Message(provider_config_cmd::run_provider_slash(args))
    }
}
