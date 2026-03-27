-- Migration: 0001_init_saas_schema
-- Description: Rollback SaaS multi-tenant database schema
-- Drop tables in reverse order (foreign key dependencies)

-- Drop domain events first (no dependencies)
DROP TABLE IF EXISTS domain_events;

-- Drop tasks (references sessions, agent_instances)
DROP TABLE IF EXISTS tasks;

-- Drop sessions (references agent_instances, users)
DROP TABLE IF EXISTS sessions;

-- Drop audit_logs (references tenants, organizations, teams, users)
DROP TABLE IF EXISTS audit_logs;

-- Drop tool_policies (references tenants, organizations, teams)
DROP TABLE IF EXISTS tool_policies;

-- Drop agent_instances (references agent_templates, tenants, organizations, teams)
DROP TABLE IF EXISTS agent_instances;

-- Drop agent_templates (references tenants)
DROP TABLE IF EXISTS agent_templates;

-- Drop memberships (references tenants, organizations, teams, users)
DROP TABLE IF EXISTS memberships;

-- Drop users (no dependencies from other tables at this point)
DROP TABLE IF EXISTS users;

-- Drop teams (references tenants, organizations)
DROP TABLE IF EXISTS teams;

-- Drop organizations (references tenants)
DROP TABLE IF EXISTS organizations;

-- Drop tenants last (base table)
DROP TABLE IF EXISTS tenants;
