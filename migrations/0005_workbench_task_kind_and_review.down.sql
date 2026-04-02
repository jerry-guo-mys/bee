DROP INDEX IF EXISTS idx_tasks_task_kind;

ALTER TABLE tasks DROP COLUMN IF EXISTS review_report_json;
ALTER TABLE tasks DROP COLUMN IF EXISTS execution_json;
ALTER TABLE tasks DROP COLUMN IF EXISTS artifacts_json;
ALTER TABLE tasks DROP COLUMN IF EXISTS task_kind;
