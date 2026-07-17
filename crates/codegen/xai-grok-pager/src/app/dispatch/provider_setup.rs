//! Interactive provider setup wizard (OpenClaude-style) → official config.toml.

use crate::app::actions::Effect;
use crate::app::app_view::{ActiveView, AppView};
use crate::provider_setup::{
    discover_models, endpoint_catalog, specs_from_selection, write_model_specs, AuthMode,
    DiscoveredModel, ProviderDraft, WizardStep, CUSTOM_ENDPOINT_ID,
};
use crate::provider_config_cmd::user_config_toml_path;
use crate::views::question_view::{LocalQuestionKind, QuestionViewState};
use xai_grok_tools::implementations::grok_build::ask_user_question::{
    Question, QuestionOption,
};

const MAX_MODEL_OPTIONS: usize = 60;

/// Open step 1: curated endpoints + Custom at the end.
pub(in crate::app::dispatch) fn open_provider_setup(app: &mut AppView) -> Vec<Effect> {
    open_step(app, WizardStep::PickEndpoint, build_pick_endpoint_question())
}

pub(in crate::app::dispatch) fn dispatch_provider_setup_answered(
    app: &mut AppView,
    step: WizardStep,
    selected: Vec<usize>,
    freeform: String,
    skipped: bool,
) -> Vec<Effect> {
    if skipped {
        app.show_toast("Provider setup cancelled");
        return vec![];
    }
    match step {
        WizardStep::PickEndpoint => {
            let catalog = endpoint_catalog();
            let idx = selected.first().copied().unwrap_or(usize::MAX);
            if idx == catalog.len() {
                // Custom
                return open_step(
                    app,
                    WizardStep::CustomDetails,
                    Question {
                        question: "Custom provider — enter:  Name | https://base-url/v1  [| backend]"
                            .into(),
                        id: None,
                        options: vec![QuestionOption {
                            label: "Continue with freeform line above".into(),
                            description: "Example: My Proxy | https://api.example.com/v1 | chat_completions"
                                .into(),
                            preview: None,
                            id: None,
                        }],
                        multi_select: Some(false),
                    },
                );
            }
            let Some(entry) = catalog.get(idx) else {
                app.show_toast("Invalid endpoint selection");
                return vec![];
            };
            let draft = ProviderDraft::from_catalog(entry);
            advance_after_draft(app, draft)
        }
        WizardStep::CustomDetails => {
            let line = freeform.trim();
            if line.is_empty() {
                app.show_toast("Enter: Name | https://base-url/v1");
                return open_provider_setup(app);
            }
            let parts: Vec<&str> = line.split('|').map(str::trim).collect();
            if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
                app.show_toast("Need Name | URL (optional | backend)");
                return open_step(
                    app,
                    WizardStep::CustomDetails,
                    Question {
                        question: "Custom provider — enter:  Name | https://base-url/v1  [| backend]"
                            .into(),
                        id: None,
                        options: vec![QuestionOption {
                            label: "Continue with freeform line above".into(),
                            description: "Example: My Proxy | https://api.example.com/v1".into(),
                            preview: None,
                            id: None,
                        }],
                        multi_select: Some(false),
                    },
                );
            }
            let backend = parts.get(2).copied().unwrap_or("chat_completions");
            let draft = ProviderDraft::custom(parts[1], parts[0], backend);
            advance_after_draft(app, draft)
        }
        WizardStep::EnterKey { mut draft } => {
            let key = freeform.trim().to_string();
            // Without a real provider key, Grok falls back to the xAI session
            // token → foreign APIs return 401 ("Auth recovery succeeded but…").
            if key.is_empty() && draft.auth == AuthMode::ApiKey {
                app.show_toast(
                    "API key required — type/paste it in the freeform (Other) row, then Enter",
                );
                return advance_after_draft(app, draft);
            }
            draft.api_key = if key.is_empty() { None } else { Some(key) };
            fetch_and_open_models(app, draft)
        }
        WizardStep::PickModels { draft, models } => {
            let mut ids: Vec<String> = selected
                .iter()
                .filter_map(|i| models.get(*i).map(|m| m.id.clone()))
                .collect();
            // Not every model fits the picker (capped at MAX_MODEL_OPTIONS) —
            // typing extra id(s) into the freeform row is the only way to
            // reach the rest, so always honor it alongside checkbox picks.
            for part in freeform.split([',', ' ', '\n']) {
                let p = part.trim();
                if !p.is_empty() {
                    ids.push(crate::provider_config_cmd::normalize_provider_model_id(p));
                }
            }
            ids.retain(|s| !s.is_empty());
            ids.dedup();
            if ids.is_empty() {
                app.show_toast(
                    "Select at least one model (Space to toggle) or type its id, then Enter",
                );
                return open_models_step(app, draft, models);
            }
            match specs_from_selection(&draft, &ids) {
                Ok(specs) => match write_model_specs(&user_config_toml_path(), &specs, true, true)
                {
                    Ok(msg) => {
                        let toast = format!(
                            "Added {} model(s) from {} — use /model",
                            specs.len(),
                            draft.provider_label
                        );
                        app.show_toast(&toast);
                        let _ = msg;
                        vec![]
                    }
                    Err(e) => {
                        let toast = format!("Write failed: {e}");
                        app.show_toast(&toast);
                        vec![]
                    }
                },
                Err(e) => {
                    app.show_toast(&e);
                    vec![]
                }
            }
        }
    }
}

fn advance_after_draft(app: &mut AppView, draft: ProviderDraft) -> Vec<Effect> {
    if draft.auth == AuthMode::None {
        return fetch_and_open_models(app, draft);
    }
    let label = draft.provider_label.clone();
    let env_hint = draft
        .env_key
        .clone()
        .unwrap_or_else(|| "API_KEY".into());
    open_step(
        app,
        WizardStep::EnterKey { draft },
        Question {
            question: format!(
                "API key for {label}\n\
                 1) Move to freeform/Other row (↓)\n\
                 2) Paste key\n\
                 3) Enter\n\
                 Stored as api_key in config.toml (also env {env_hint}).\n\
                 Empty key → 401 Auth recovery (xAI token sent to foreign API)."
            ),
            id: None,
            options: vec![QuestionOption {
                label: "I pasted the key in freeform — continue".into(),
                description: format!("Required — without it {label} will 401"),
                preview: None,
                id: None,
            }],
            multi_select: Some(false),
        },
    )
}

fn fetch_and_open_models(app: &mut AppView, draft: ProviderDraft) -> Vec<Effect> {
    let fetching = format!("Fetching models from {}…", draft.base_url);
    app.show_toast(&fetching);
    // Brief blocking fetch (setup path only; 20s timeout in client).
    match discover_models(&draft) {
        Ok(models) => open_models_step(app, draft, models),
        Err(e) => {
            let toast = format!("Model list failed: {e}");
            app.show_toast(&toast);
            // Still offer default if any.
            if !draft.default_model.is_empty() {
                let m = DiscoveredModel {
                    id: draft.default_model.clone(),
                    display: draft.default_model.clone(),
                };
                open_models_step(app, draft, vec![m])
            } else {
                vec![]
            }
        }
    }
}

fn open_models_step(
    app: &mut AppView,
    draft: ProviderDraft,
    models: Vec<DiscoveredModel>,
) -> Vec<Effect> {
    let provider = draft.provider_label.clone();
    let show: Vec<&DiscoveredModel> = models.iter().take(MAX_MODEL_OPTIONS).collect();
    let options: Vec<QuestionOption> = show
        .iter()
        .map(|m| QuestionOption {
            label: m.display.clone(),
            description: provider.clone(),
            preview: None,
            id: Some(m.id.clone()),
        })
        .collect();
    let more = if models.len() > MAX_MODEL_OPTIONS {
        format!(
            " (showing first {MAX_MODEL_OPTIONS} of {} — not listed? type its id in the text box below)",
            models.len()
        )
    } else {
        String::new()
    };
    // Truncate models stored in step to match options indices.
    let models_stored: Vec<DiscoveredModel> = show.into_iter().cloned().collect();
    open_step(
        app,
        WizardStep::PickModels {
            draft,
            models: models_stored,
        },
        Question {
            question: format!(
                "Select models to add{more}. Provider name appears on the right of /model."
            ),
            id: None,
            options,
            multi_select: Some(true),
        },
    )
}

fn build_pick_endpoint_question() -> Question {
    let mut options: Vec<QuestionOption> = endpoint_catalog()
        .iter()
        .map(|e| QuestionOption {
            label: e.label.to_string(),
            description: e.blurb.to_string(),
            preview: None,
            id: Some(e.id.to_string()),
        })
        .collect();
    options.push(QuestionOption {
        label: "Custom provider…".into(),
        description: "Enter base URL + display name".into(),
        preview: None,
        id: Some(CUSTOM_ENDPOINT_ID.to_string()),
    });
    Question {
        question: "Add provider — pick an endpoint (OpenClaude-style). Right column = details."
            .into(),
        id: None,
        options,
        multi_select: Some(false),
    }
}

fn open_step(app: &mut AppView, step: WizardStep, question: Question) -> Vec<Effect> {
    let ActiveView::Agent(id) = app.active_view else {
        app.show_toast("Open a session first, then /provider setup");
        return vec![];
    };
    let Some(agent) = app.agents.get_mut(&id) else {
        return vec![];
    };
    if agent.question_view.is_some() {
        // Replace in-progress local wizard; refuse if ACP question.
        if agent
            .question_view
            .as_ref()
            .is_some_and(|q| q.local_kind.is_none())
        {
            app.show_toast("Finish answering the current question first");
            return vec![];
        }
    }
    let stashed = agent.prompt.stash();
    let state = QuestionViewState::new(
        format!("provider-setup-{}", uuid::Uuid::new_v4()),
        vec![question],
        stashed,
    )
    .with_local_kind(LocalQuestionKind::ProviderSetup { step });
    agent.question_view = Some(state);
    agent.prompt.set_text("");
    vec![]
}
