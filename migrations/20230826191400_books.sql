ALTER TABLE books DROP COLUMN series;
ALTER TABLE books ADD COLUMN series INT4 NOT NULL REFERENCES book_series ON DELETE CASCADE;