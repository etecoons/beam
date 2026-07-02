//! Wire-format / shared data types used by both the backend and frontend.
//!
//! These types form the on-the-wire and on-disk contract between the
//! Yew frontend and the Axum backend. They live in `shared-core` so
//! that they are available to both crates without a circular
//! dependency on `shared-backend` or `shared-frontend`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

// ──────────────────────────── domain types ─────────────────────────────

/// A single todo item, as stored on disk and rendered in the frontend.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoItem {
    /// Stable identifier, generated client-side or by the backend
    /// migration. Used as the React/Yew key so that
    /// reorder/edit/delete operations are O(1).
    #[serde(default)]
    pub id: String,
    /// The task text. May contain any UTF-8.
    pub text: String,
    /// Whether the task has been completed.
    pub completed: bool,
}

/// On-disk and over-the-wire representation: `list_name -> ordered tasks`.
pub type TodoLists = HashMap<String, Vec<TodoItem>>;

// ──────────────────────── API contract types ───────────────────────────

/// Public site configuration. Returned by `GET /api/config` and used by
/// the frontend to render the header, theme picker, etc.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteConfig {
    pub site_title: String,
    pub single_list: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
    #[serde(default = "default_true")]
    pub show_version: bool,
    #[serde(default = "default_true")]
    pub show_github: bool,
}

/// Status of PIN authentication on the server. Returned by
/// `GET /api/pin-required` so the login screen can render the correct
/// prompts and disable itself when no PIN is configured.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PinRequiredResponse {
    pub required: bool,
    pub length: usize,
    pub locked: bool,
    pub attempts_left: usize,
    pub lockout_minutes: u64,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
    #[serde(default = "default_true")]
    pub show_version: bool,
    #[serde(default = "default_true")]
    pub show_github: bool,
}

/// Request body for `POST /api/verify-pin`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerifyPinRequest {
    pub pin: String,
}

/// Response body for `POST /api/verify-pin`.
///
/// `error`, `attempts_left`, `locked`, `lockout_minutes` are populated
/// on failure and omitted on success so that the success case is the
/// minimal `{"valid": true}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPinResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts_left: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockout_minutes: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_item_default_id_is_empty() {
        let item = TodoItem {
            id: String::new(),
            text: "x".into(),
            completed: false,
        };
        assert_eq!(item.id, "");
    }

    #[test]
    fn todo_lists_roundtrip() {
        let mut lists: TodoLists = HashMap::new();
        lists.insert(
            "inbox".to_string(),
            vec![TodoItem {
                id: "abc123".into(),
                text: "buy milk".into(),
                completed: false,
            }],
        );
        let json = serde_json::to_string(&lists).unwrap();
        let back: TodoLists = serde_json::from_str(&json).unwrap();
        assert_eq!(lists, back);
    }

    #[test]
    fn verify_pin_response_success_is_minimal() {
        let resp = VerifyPinResponse {
            valid: true,
            error: None,
            attempts_left: None,
            locked: None,
            lockout_minutes: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(json, r#"{"valid":true}"#);
    }

    #[test]
    fn site_config_defaults_show_flags_true() {
        let json =
            r#"{"siteTitle":"x","singleList":false,"enableThemes":false,"enablePrint":false}"#;
        let cfg: SiteConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.show_version);
        assert!(cfg.show_github);
    }
}
