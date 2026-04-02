DROP INDEX IF EXISTS idx_tasks_team_status;
DROP INDEX IF EXISTS idx_tasks_workflow_run_id;

ALTER TABLE tasks DROP COLUMN IF EXISTS internal_group;
ALTER TABLE tasks DROP COLUMN IF EXISTS coordinator_id;
ALTER TABLE tasks DROP COLUMN IF EXISTS group_id;
ALTER TABLE tasks DROP COLUMN IF EXISTS assignee_ids_json;
ALTER TABLE tasks DROP COLUMN IF EXISTS workflow_template_version;
ALTER TABLE tasks DROP COLUMN IF EXISTS workflow_template_id;
ALTER TABLE tasks DROP COLUMN IF EXISTS workflow_run_id;
