-- 0002_task_lifecycle.sql
ALTER TABLE tasks ADD COLUMN description TEXT;
UPDATE tasks SET description = 'No description provided.' WHERE description IS NULL;
ALTER TABLE tasks ALTER COLUMN description SET NOT NULL;

ALTER TABLE tasks ADD COLUMN status TEXT;
UPDATE tasks
SET status = CASE WHEN is_done THEN 'COMPLETED' ELSE 'PLANNED' END
WHERE status IS NULL;
ALTER TABLE tasks ALTER COLUMN status SET NOT NULL;

ALTER TABLE tasks ADD COLUMN updated_at TIMESTAMPTZ;
UPDATE tasks SET updated_at = created_at WHERE updated_at IS NULL;
ALTER TABLE tasks ALTER COLUMN updated_at SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN updated_at SET DEFAULT now();

ALTER TABLE tasks ADD CONSTRAINT tasks_description_length CHECK (char_length(description) BETWEEN 1 AND 4000);
ALTER TABLE tasks ADD CONSTRAINT tasks_status_valid CHECK (status IN ('PLANNED', 'IN_PROGRESS', 'COMPLETED'));
ALTER TABLE tasks ADD CONSTRAINT tasks_updated_after_created CHECK (updated_at >= created_at);

ALTER TABLE tasks DROP COLUMN is_done;

DROP INDEX IF EXISTS idx_tasks_is_done;
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status);
CREATE INDEX IF NOT EXISTS idx_tasks_updated_at ON tasks (updated_at DESC);
