-- Create project shares table for sharing projects with different access levels
CREATE TABLE project_shares (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    share_token VARCHAR(255) UNIQUE NOT NULL,
    share_type VARCHAR(50) NOT NULL CHECK (share_type IN ('new_chat', 'all_history', 'specific_conversations')),
    
    -- Settings stored as JSONB for flexibility
    settings JSONB NOT NULL DEFAULT '{}',
    
    -- Access control
    is_public BOOLEAN NOT NULL DEFAULT true,
    is_read_only BOOLEAN NOT NULL DEFAULT false,
    max_messages_per_session INTEGER,
    
    -- Expiration
    expires_at TIMESTAMP,
    
    -- Audit fields
    created_by UUID,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMP,
    
    -- Usage tracking
    view_count INTEGER DEFAULT 0,
    last_accessed_at TIMESTAMP
);

-- Create table for tracking specific conversations in a share
CREATE TABLE project_share_conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_share_id UUID NOT NULL REFERENCES project_shares(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    UNIQUE(project_share_id, conversation_id)
);

-- Create table for tracking share access sessions
CREATE TABLE project_share_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_share_id UUID NOT NULL REFERENCES project_shares(id) ON DELETE CASCADE,
    session_token VARCHAR(255) UNIQUE NOT NULL,
    
    -- Session info
    user_agent TEXT,
    ip_address INET,
    referrer TEXT,
    
    -- Session activity
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_activity_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
    
    -- Message tracking for this session
    message_count INTEGER DEFAULT 0,
    max_messages INTEGER DEFAULT 50
);

-- Create indexes for performance
CREATE INDEX idx_project_shares_token ON project_shares(share_token) WHERE deleted_at IS NULL;
CREATE INDEX idx_project_shares_project_id ON project_shares(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_project_shares_expires_at ON project_shares(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_project_shares_created_by ON project_shares(created_by);

CREATE INDEX idx_share_conversations_share_id ON project_share_conversations(project_share_id);
CREATE INDEX idx_share_conversations_conversation_id ON project_share_conversations(conversation_id);

CREATE INDEX idx_share_sessions_token ON project_share_sessions(session_token);
CREATE INDEX idx_share_sessions_share_id ON project_share_sessions(project_share_id);
CREATE INDEX idx_share_sessions_expires_at ON project_share_sessions(expires_at);

-- Add comments for documentation
COMMENT ON TABLE project_shares IS 'Stores sharing configurations for projects';
COMMENT ON COLUMN project_shares.share_type IS 'Type of sharing: new_chat (blank), all_history (full), specific_conversations (selected)';
COMMENT ON COLUMN project_shares.settings IS 'JSON configuration: theme, features, branding, etc.';
COMMENT ON COLUMN project_shares.is_read_only IS 'If true, visitors can only view, not send messages';
COMMENT ON COLUMN project_shares.max_messages_per_session IS 'Max messages per session (null = unlimited)';

COMMENT ON TABLE project_share_conversations IS 'Links specific conversations to a share when share_type=specific_conversations';
COMMENT ON TABLE project_share_sessions IS 'Tracks individual visitor sessions to shared projects';