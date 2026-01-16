-- Auth session support (refresh token rotation)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS refresh_jti TEXT;

