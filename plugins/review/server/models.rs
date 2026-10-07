#[derive(Debug, sqlx::FromRow)]
pub(super) struct ReviewCriterionRow {
    pub id: String,
    pub title: String,
    pub review_question: String,
    pub display_order: i16,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct ReviewSummaryCountsRow {
    pub total: i64,
    pub completed: i64,
    pub satisfied: i64,
    pub needs_information: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct ReviewListItemRow {
    pub id: String,
    pub criterion_id: String,
    pub criterion_title: String,
    pub status: String,
    pub opinion: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct ReviewDetailJoinRow {
    pub id: String,
    pub criterion_id: String,
    pub criterion_title: String,
    pub review_question: String,
    pub status: String,
    pub opinion: String,
    pub created_at: String,
    pub updated_at: String,
    pub evidence_id: Option<String>,
    pub evidence_title: Option<String>,
    pub evidence_file_name: Option<String>,
    pub evidence_status: Option<String>,
}
