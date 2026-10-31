use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::sharepoint::inner::content_type_id;

/// Selects which changes `get_changes` returns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChangeQuery {
    /// Include additions.
    pub add: bool,
    /// Include updates.
    pub update: bool,
    /// Include deletions.
    pub delete_object: bool,
    /// Include item changes.
    pub item: bool,
    /// Resume after this token (from a previous [`Change::change_token`]).
    pub change_token_start: Option<String>,
    /// Stop at this token.
    pub change_token_end: Option<String>,
}

impl ChangeQuery {
    /// Item additions, updates and deletions.
    pub fn items() -> Self {
        Self {
            add: true,
            update: true,
            delete_object: true,
            item: true,
            ..Self::default()
        }
    }

    /// Starts after the given change token.
    pub fn since(mut self, token: impl Into<String>) -> Self {
        self.change_token_start = Some(token.into());
        self
    }

    pub(crate) fn to_body(&self) -> Value {
        let mut q = Map::new();
        q.insert(
            "__metadata".into(),
            serde_json::json!({"type":"SP.ChangeQuery"}),
        );
        q.insert("Add".into(), self.add.into());
        q.insert("Update".into(), self.update.into());
        q.insert("DeleteObject".into(), self.delete_object.into());
        q.insert("Item".into(), self.item.into());
        for (key, token) in [
            ("ChangeTokenStart", &self.change_token_start),
            ("ChangeTokenEnd", &self.change_token_end),
        ] {
            if let Some(t) = token {
                q.insert(
                    key.into(),
                    serde_json::json!({"__metadata":{"type":"SP.ChangeToken"},"StringValue":t}),
                );
            }
        }
        serde_json::json!({ "query": q })
    }
}

/// A change reported by `get_changes`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Change {
    /// Change type (1 add, 2 update, 3 delete, ...).
    #[serde(default)]
    pub change_type: i32,
    /// Token of this change; pass it to [`ChangeQuery::since`] to resume.
    #[serde(default, deserialize_with = "content_type_id")]
    pub change_token: String,
    /// Changed item id, for item changes.
    #[serde(default)]
    pub item_id: Option<i64>,
    /// Id of the list the change belongs to.
    #[serde(default)]
    pub list_id: Option<String>,
    /// Time of the change (ISO 8601).
    #[serde(default)]
    pub time: Option<String>,
}
