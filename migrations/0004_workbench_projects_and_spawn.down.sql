DROP INDEX IF EXISTS idx_task_spawn_parent;
DROP TABLE IF EXISTS task_spawn_idempotency;

DROP INDEX IF EXISTS idx_tasks_parent;
DROP INDEX IF EXISTS idx_tasks_project;
ALTER TABLE tasks DROP COLUMN IF EXISTS parent_task_id;
ALTER TABLE tasks DROP COLUMN IF EXISTS project_id;

DROP INDEX IF EXISTS idx_projects_org;
DROP TABLE IF EXISTS projects;
