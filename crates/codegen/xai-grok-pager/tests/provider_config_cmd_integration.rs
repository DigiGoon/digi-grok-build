//! Integration tests for official config.toml provider helpers.

use std::fs;
use tempfile::tempdir;
use xai_grok_pager::config_toml_edit::read_config_document_for_edit;
use xai_grok_pager::provider_config_cmd::{list_model_ids, upsert_model_table};

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
    assert!(body.contains("nvidia"));
    assert!(body.contains("z-ai/glm-5.2"));
    assert!(body.contains("NVIDIA_API_KEY"));
    assert!(body.contains("theme"));
    let doc = read_config_document_for_edit(&path).unwrap();
    assert!(list_model_ids(&doc).contains(&"nvidia".to_string()));
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
