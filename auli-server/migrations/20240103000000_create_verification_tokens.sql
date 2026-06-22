CREATE TABLE verification_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast verification link validation
CREATE INDEX idx_verification_tokens_token_hash_valid 
    ON verification_tokens(token_hash);

-- Optional: find pending verifications per user
CREATE INDEX idx_verification_tokens_user_id_expires 
    ON verification_tokens(user_id, expires_at DESC);

-- Cleanup support
CREATE INDEX idx_verification_tokens_expired 
    ON verification_tokens(expires_at);
