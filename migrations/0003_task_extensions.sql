-- 0003_task_extensions.sql
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN;
UPDATE tasks SET is_deleted = FALSE WHERE is_deleted IS NULL;
ALTER TABLE tasks ALTER COLUMN is_deleted SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN is_deleted SET DEFAULT FALSE;

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS due_at TIMESTAMPTZ;

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS priority SMALLINT;
UPDATE tasks SET priority = 3 WHERE priority IS NULL;
ALTER TABLE tasks ALTER COLUMN priority SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN priority SET DEFAULT 3;

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS row_version BIGINT;
UPDATE tasks SET row_version = 1 WHERE row_version IS NULL;
ALTER TABLE tasks ALTER COLUMN row_version SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN row_version SET DEFAULT 1;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'tasks_deleted_consistency'
    ) THEN
        ALTER TABLE tasks
            ADD CONSTRAINT tasks_deleted_consistency
            CHECK (
                (is_deleted = FALSE AND deleted_at IS NULL)
                OR
                (is_deleted = TRUE AND deleted_at IS NOT NULL)
            );
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'tasks_priority_range'
    ) THEN
        ALTER TABLE tasks
            ADD CONSTRAINT tasks_priority_range
            CHECK (priority BETWEEN 1 AND 5);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tasks_active_updated_at
ON tasks (updated_at DESC)
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_tasks_active_status
ON tasks (status)
WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_tasks_due_at
ON tasks (due_at)
WHERE is_deleted = FALSE;
