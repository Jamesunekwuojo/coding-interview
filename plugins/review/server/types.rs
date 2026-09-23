use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ReviewHealthResponse {
    pub status: String,
    pub plugin_id: String,
}
