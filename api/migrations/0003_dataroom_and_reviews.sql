-- Materials provided by the company inside a DataRoom.
CREATE TABLE materials (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    uploader_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    file_name TEXT NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ready',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT materials_file_name_valid
        CHECK (file_name ~* '\.(txt|md)$'),

    CONSTRAINT materials_status_valid
        CHECK (status IN ('ready', 'processing', 'failed'))
);

CREATE INDEX materials_workspace_created_idx
    ON materials (workspace_id, created_at DESC, id ASC);


-- An investor's review of one fixed review criterion.
CREATE TABLE reviews (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    criterion_id TEXT NOT NULL REFERENCES review_criteria(id),
    status TEXT NOT NULL,
    opinion TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT reviews_status_valid
        CHECK (status IN ('satisfied', 'needs_information')),

    CONSTRAINT reviews_opinion_valid
        CHECK (
            length(trim(opinion)) >= 1
            AND length(opinion) <= 2000
        ),

    CONSTRAINT reviews_workspace_user_criterion_unique
        UNIQUE (workspace_id, criterion_id, user_id)
);

CREATE INDEX reviews_workspace_user_idx
    ON reviews (workspace_id, user_id);


-- Many-to-many relationship between reviews and evidence materials.
CREATE TABLE review_evidence (
    review_id TEXT NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
    material_id TEXT NOT NULL REFERENCES materials(id) ON DELETE CASCADE,

    PRIMARY KEY (review_id, material_id)
);

CREATE INDEX review_evidence_material_idx
    ON review_evidence (material_id);


-- Sample DataRoom materials from samples/scenario.json.
INSERT INTO materials (
    id,
    workspace_id,
    uploader_id,
    title,
    file_name,
    content,
    status
) VALUES
(
    'mat-doc-business',
    'lighthouse',
    'company-user',
    '회사 소개',
    'company-overview.md',
    '제조사 재고 관리 구독형 소프트웨어. 사업장당 월 15만 원, 2026년 8월 유료 고객 40개.',
    'ready'
),
(
    'mat-doc-team',
    'lighthouse',
    'company-user',
    '팀 소개',
    'team.md',
    '대표 제조업 운영 8년, 개발 책임자 B2B 개발 6년, 디자이너 4년. 전담 영업 담당자는 없음.',
    'ready'
),
(
    'mat-doc-revenue',
    'lighthouse',
    'company-user',
    '매출 자료',
    'revenue.txt',
    '자료를 읽지 못했습니다.',
    'failed'
),
(
    'mat-doc-pipeline',
    'lighthouse',
    'company-user',
    '고객 인터뷰',
    'customer-interviews.md',
    '아직 준비되지 않았습니다.',
    'processing'
);