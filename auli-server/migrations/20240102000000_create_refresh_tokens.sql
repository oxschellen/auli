CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Critical: Fast lookup when validating a refresh token
CREATE INDEX idx_refresh_tokens_token_hash_active 
    ON refresh_tokens(token_hash) 
    WHERE revoked = FALSE;

-- For revoking all tokens of a user (on logout/password change)
CREATE INDEX idx_refresh_tokens_user_id_revoked 
    ON refresh_tokens(user_id) 
    WHERE revoked = FALSE;

-- For cleanup jobs
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at) 
    WHERE revoked = FALSE;
