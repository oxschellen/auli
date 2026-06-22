CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Primary lookup: email (used in login, register, password reset)
CREATE INDEX idx_users_email_lower ON users(LOWER(email));

-- Optional: if you frequently query unverified users (e.g., resend verification)
CREATE INDEX idx_users_unverified ON users(is_verified) WHERE is_verified = FALSE;

-- For user activity analytics
CREATE INDEX idx_users_created_at ON users(created_at DESC);