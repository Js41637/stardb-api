CREATE TABLE IF NOT EXISTS verifications (
    uid INT8 PRIMARY KEY NOT NULL REFERENCES mihomo ON DELETE CASCADE,
    username TEXT NOT NULL REFERENCES users ON DELETE CASCADE,
    token TEXT NOT NULL
);