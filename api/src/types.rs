use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub enum UserRole {
    Company,
    Investor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct Viewer {
    pub id: String,
    pub name: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct HostSession {
    pub user: Viewer,
    pub workspace: Workspace,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct GetSessionResponse {
    pub session: Option<HostSession>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct LogoutResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct PluginRegistration {
    pub id: String,
    pub name: String,
    pub manifest_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct DataroomRpcRequest {
    pub workspace_id: String,
    pub method: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-bridge", ts(type = "unknown"))]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct PluginRpcRequest {
    pub plugin_id: String,
    pub workspace_id: String,
    pub method: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-bridge", ts(type = "unknown"))]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct RpcResponse {
    #[cfg_attr(feature = "ts-bridge", ts(type = "unknown"))]
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts-bridge",
    derive(ts_rs::TS),
    ts(export, export_to = "types/")
)]
pub struct ApiErrorBody {
    pub kind: String,
    pub message: String,
}
