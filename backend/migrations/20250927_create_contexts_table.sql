-- Create contexts table for individual programmable context items
CREATE TABLE IF NOT EXISTS contexts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    context_type VARCHAR(50) NOT NULL CHECK (context_type IN ('manual', 'script', 'sql_query', 'api_call')),
    source_content TEXT NOT NULL,
    source_language VARCHAR(50),
    execution_type VARCHAR(50) CHECK (execution_type IN ('node', 'python', 'bash', 'sql')),
    execution_timeout INTEGER,
    refresh_interval INTEGER,
    compile_settings JSONB,
    tags TEXT[],
    position INTEGER,
    compiled_content TEXT,
    compiled_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    UNIQUE(project_id, name)
);

-- Create index for efficient querying
CREATE INDEX IF NOT EXISTS idx_contexts_project_id ON contexts(project_id);
CREATE INDEX IF NOT EXISTS idx_contexts_active ON contexts(project_id, is_active) WHERE deleted_at IS NULL;