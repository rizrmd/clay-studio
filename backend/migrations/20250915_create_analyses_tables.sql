-- Create analyses table
CREATE TABLE IF NOT EXISTS analyses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR NOT NULL,
    script_content TEXT NOT NULL,
    project_id CHARACTER VARYING NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    version INTEGER DEFAULT 1,
    is_active BOOLEAN DEFAULT TRUE,
    metadata JSONB DEFAULT '{}'
);

-- Create analysis versions table for history tracking
CREATE TABLE IF NOT EXISTS analysis_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID NOT NULL REFERENCES analyses(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    script_content TEXT NOT NULL,
    change_description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by VARCHAR DEFAULT 'mcp',
    metadata JSONB DEFAULT '{}'
);

-- Create analysis schedules table
CREATE TABLE IF NOT EXISTS analysis_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID NOT NULL REFERENCES analyses(id) ON DELETE CASCADE,
    cron_expression VARCHAR NOT NULL,
    timezone VARCHAR DEFAULT 'UTC',
    enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_run_at TIMESTAMP WITH TIME ZONE,
    next_run_at TIMESTAMP WITH TIME ZONE
);

-- Create analysis jobs table for execution tracking
CREATE TABLE IF NOT EXISTS analysis_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID NOT NULL REFERENCES analyses(id) ON DELETE CASCADE,
    status VARCHAR NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, cancelled
    parameters JSONB DEFAULT '{}',
    result JSONB,
    error_message TEXT,
    logs TEXT[],
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    execution_time_ms BIGINT,
    triggered_by VARCHAR DEFAULT 'manual' -- manual, schedule, api
);

-- Create analysis dependencies table
CREATE TABLE IF NOT EXISTS analysis_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID NOT NULL REFERENCES analyses(id) ON DELETE CASCADE,
    dependency_type VARCHAR NOT NULL, -- datasource, analysis
    dependency_name VARCHAR NOT NULL,
    dependency_id UUID, -- for analysis dependencies
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create analysis results storage table for metadata
CREATE TABLE IF NOT EXISTS analysis_result_storage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES analysis_jobs(id) ON DELETE CASCADE,
    storage_path VARCHAR NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum VARCHAR,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_analyses_project_id ON analyses(project_id);
CREATE INDEX IF NOT EXISTS idx_analyses_active ON analyses(is_active) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_analysis_versions_analysis_id ON analysis_versions(analysis_id);
CREATE INDEX IF NOT EXISTS idx_analysis_schedules_analysis_id ON analysis_schedules(analysis_id);
CREATE INDEX IF NOT EXISTS idx_analysis_schedules_next_run ON analysis_schedules(next_run_at) WHERE enabled = true;
CREATE INDEX IF NOT EXISTS idx_analysis_jobs_analysis_id ON analysis_jobs(analysis_id);
CREATE INDEX IF NOT EXISTS idx_analysis_jobs_status ON analysis_jobs(status);
CREATE INDEX IF NOT EXISTS idx_analysis_jobs_created_at ON analysis_jobs(created_at);
CREATE INDEX IF NOT EXISTS idx_analysis_dependencies_analysis_id ON analysis_dependencies(analysis_id);
CREATE INDEX IF NOT EXISTS idx_analysis_result_storage_job_id ON analysis_result_storage(job_id);

-- Create unique constraint for analysis versions
CREATE UNIQUE INDEX IF NOT EXISTS idx_analysis_versions_unique ON analysis_versions(analysis_id, version_number);

-- Add triggers for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_analyses_updated_at 
    BEFORE UPDATE ON analyses
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_analysis_schedules_updated_at 
    BEFORE UPDATE ON analysis_schedules
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();