-- Add context field to projects table
ALTER TABLE projects 
ADD COLUMN IF NOT EXISTS context TEXT,
ADD COLUMN IF NOT EXISTS context_compiled TEXT,
ADD COLUMN IF NOT EXISTS context_compiled_at TIMESTAMPTZ;