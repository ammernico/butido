-- This file should undo anything in `up.sql`
ALTER TABLE
    artifacts
ADD CONSTRAINT
    artifacts_path_key -- as generated by default for postgresql
    UNIQUE (path)
