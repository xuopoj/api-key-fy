-- Users (local record of Huawei IAM users who have logged in)
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    iam_username TEXT NOT NULL UNIQUE,
    iam_domain TEXT NOT NULL,
    display_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

-- Sessions (cookie-based admin sessions)
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Credentials (Huawei IAM credentials for upstream auth)
CREATE TABLE credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_shared BOOLEAN NOT NULL DEFAULT FALSE,
    iam_username TEXT NOT NULL,
    iam_password_enc TEXT NOT NULL,  -- encrypted at rest
    iam_domain TEXT NOT NULL,
    iam_project TEXT NOT NULL,
    iam_region TEXT NOT NULL,
    iam_endpoint TEXT,  -- custom IAM endpoint for private cloud; if NULL, defaults to https://iam.{region}.myhuaweicloud.com
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Apps (proxy entries)
CREATE TABLE apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT UNIQUE,                -- for http_path type
    type TEXT NOT NULL CHECK (type IN ('http_path', 'http_port')),
    credential_id UUID NOT NULL REFERENCES credentials(id),
    base_url TEXT NOT NULL,
    upstream_base_path TEXT NOT NULL DEFAULT '/',
    -- http_path specific
    strip_prefix BOOLEAN NOT NULL DEFAULT TRUE,
    -- http_port specific
    listen_port INT,
    -- http_port access control
    user_whitelist TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API Keys
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,   -- bcrypt hash of the actual key
    key_prefix TEXT NOT NULL,        -- first 8 chars for display (e.g. "akfy_abc1")
    credential_id UUID NOT NULL REFERENCES credentials(id),
    expires_at TIMESTAMPTZ,          -- NULL = never expires
    rate_limit_rpm INT NOT NULL DEFAULT 60,
    logging_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API Key <-> App (many-to-many)
CREATE TABLE api_key_apps (
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    app_id UUID NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    PRIMARY KEY (api_key_id, app_id)
);

-- Request Logs
CREATE TABLE request_logs (
    id BIGSERIAL PRIMARY KEY,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    app_id UUID REFERENCES apps(id) ON DELETE SET NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status_code INT,
    latency_ms INT,
    request_size_bytes BIGINT,
    response_size_bytes BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_request_logs_api_key_id ON request_logs(api_key_id);
CREATE INDEX idx_request_logs_app_id ON request_logs(app_id);
CREATE INDEX idx_request_logs_created_at ON request_logs(created_at);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
