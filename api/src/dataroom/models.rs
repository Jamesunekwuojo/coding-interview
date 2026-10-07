#[derive(Debug, sqlx::FromRow)]
pub(super) struct MaterialSummaryRow {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct MaterialDetailRow {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub status: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}
