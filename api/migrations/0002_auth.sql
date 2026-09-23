CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    CONSTRAINT users_email_normalized CHECK (email = LOWER(email))
);

CREATE TABLE workspace_users (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('company', 'investor')),
    PRIMARY KEY (workspace_id, user_id)
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

INSERT INTO workspaces (id, name) VALUES
    ('lighthouse', '라이트하우스');

INSERT INTO users (id, email, display_name, password_hash) VALUES
    ('company-user', 'company@lighthouse.test', '김서연', '$argon2id$v=19$m=19456,t=2,p=1$Y29tcGFueS1kYXRhcm9vbQ$GBF/Ud/5JezbwxbysSGkwBqB2sgApJtEmockgLicJkw'),
    ('investor-user', 'investor@lighthouse.test', '이도윤', '$argon2id$v=19$m=19456,t=2,p=1$aW52ZXN0b3ItZGF0YXJvb20$7XwQ4P+RYJYwrPwRxdZk/4tRQ9FGl3/N2DQUjzUzBGQ'),
    ('investor-peer', 'peer@lighthouse.test', '박지우', '$argon2id$v=19$m=19456,t=2,p=1$cGVlci1kYXRhcm9vbQ$0O0HvXv5tgef2IOAarPk3Q3VmT89Vc0zbw/jqPgAfFw');

INSERT INTO workspace_users (workspace_id, user_id, role) VALUES
    ('lighthouse', 'company-user', 'company'),
    ('lighthouse', 'investor-user', 'investor'),
    ('lighthouse', 'investor-peer', 'investor');
