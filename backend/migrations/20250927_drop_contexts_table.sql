-- Drop the unused contexts table
DROP TABLE IF EXISTS contexts;

-- Drop associated indexes
DROP INDEX IF EXISTS idx_contexts_project_id;
DROP INDEX IF EXISTS idx_contexts_active;