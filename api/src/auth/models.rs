#[derive(Debug, sqlx::FromRow)]
pub(super) struct SessionUserRow {
    pub(super) id: String,
    pub(super) display_name: String,
    pub(super) role: String,
    pub(super) workspace_id: String,
    pub(super) workspace_name: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct LoginUserRow {
    pub(crate) id: String,
    pub(crate) password_hash: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) workspace_id: String,
    pub(crate) workspace_name: String,
}
