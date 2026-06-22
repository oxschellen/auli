CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Critical: Fast reset link validation
CREATE INDEX idx_password_reset_tokens_token_hash_valid 
    ON password_reset_tokens(token_hash) 
    WHERE used = FALSE;

-- Revoke old tokens per user on new request
CREATE INDEX idx_password_reset_tokens_user_id_unused 
    ON password_reset_tokens(user_id) 
    WHERE used = FALSE;

-- Cleanup
CREATE INDEX idx_password_reset_tokens_cleanup 
    ON password_reset_tokens(expires_at, used);
