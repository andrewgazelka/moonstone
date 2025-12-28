-- Enrollments table
CREATE TABLE enrollments (
    id TEXT PRIMARY KEY NOT NULL,
    enroll_type TEXT NOT NULL,
    device_id TEXT,
    parent_id TEXT,
    topic TEXT NOT NULL,
    push_magic TEXT,
    push_token BLOB,
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    authenticate_raw BLOB,
    token_update_raw BLOB,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_enrollments_parent ON enrollments(parent_id);
CREATE INDEX idx_enrollments_disabled ON enrollments(disabled);

-- Commands queue
CREATE TABLE commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    enrollment_id TEXT NOT NULL REFERENCES enrollments(id),
    uuid TEXT UNIQUE NOT NULL,
    command BLOB NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending',
    result BLOB,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_commands_enrollment ON commands(enrollment_id);
CREATE INDEX idx_commands_status ON commands(status);

-- Push certificates
CREATE TABLE push_certs (
    topic TEXT PRIMARY KEY NOT NULL,
    cert_pem TEXT NOT NULL,
    key_pem TEXT NOT NULL,
    not_after TIMESTAMP
);

-- Bootstrap tokens
CREATE TABLE bootstrap_tokens (
    enrollment_id TEXT PRIMARY KEY NOT NULL REFERENCES enrollments(id),
    token BLOB NOT NULL
);

-- Certificate authentication
CREATE TABLE cert_auth (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    enrollment_id TEXT NOT NULL,
    cert_hash BLOB NOT NULL
);

CREATE INDEX idx_cert_auth_enrollment ON cert_auth(enrollment_id);
CREATE INDEX idx_cert_auth_hash ON cert_auth(cert_hash);
