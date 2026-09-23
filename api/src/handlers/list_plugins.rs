use crate::{auth::CurrentUser, plugins, types::PluginRegistration};
use ts_server_fn::get;

#[get("/api/plugins")]
pub async fn list_plugins_handler(_user: CurrentUser) -> Vec<PluginRegistration> {
    plugins::catalog()
}
