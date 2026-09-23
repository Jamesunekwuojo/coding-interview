CREATE TABLE review_criteria (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    review_question TEXT NOT NULL,
    display_order SMALLINT NOT NULL UNIQUE
);

INSERT INTO review_criteria (id, title, review_question, display_order) VALUES
    ('business', '사업 이해', '사업 모델과 고객·시장에 관한 핵심 내용이 자료로 확인되는가?', 1),
    ('team', '팀 구성', '핵심 역할과 이를 수행할 팀의 역량이 자료로 확인되는가?', 2),
    ('revenue', '매출 현황', '매출 규모·기간·추세를 자료로 확인할 수 있는가?', 3);
