-- Create project_members table for multi-user project access
CREATE TABLE IF NOT EXISTS project_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL,
    role VARCHAR(50) NOT NULL CHECK (role IN ('owner', 'member')),
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_project_members_project_id ON project_members(project_id);
CREATE INDEX IF NOT EXISTS idx_project_members_user_id ON project_members(user_id);
CREATE INDEX IF NOT EXISTS idx_project_members_role ON project_members(project_id, role);

-- Add columns to conversations table for visibility control
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'conversations' AND column_name = 'created_by_user_id') THEN
        ALTER TABLE conversations ADD COLUMN created_by_user_id UUID;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'conversations' AND column_name = 'visibility') THEN
        ALTER TABLE conversations ADD COLUMN visibility VARCHAR(50) DEFAULT 'private' CHECK (visibility IN ('private', 'public'));
    END IF;
END $$;

-- Create index for conversation filtering
CREATE INDEX IF NOT EXISTS idx_conversations_visibility ON conversations(project_id, visibility);
CREATE INDEX IF NOT EXISTS idx_conversations_created_by ON conversations(created_by_user_id);

-- Backfill existing data: migrate current project owners to project_members table
INSERT INTO project_members (project_id, user_id, role, joined_at, created_at)
SELECT p.id, p.user_id, 'owner', p.created_at, p.created_at
FROM projects p
WHERE p.user_id IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM project_members pm
    WHERE pm.project_id = p.id AND pm.user_id = p.user_id
  );

-- Backfill conversations: set created_by_user_id to project owner for existing conversations
UPDATE conversations c
SET created_by_user_id = p.user_id
FROM projects p
WHERE c.project_id = p.id
  AND c.created_by_user_id IS NULL
  AND p.user_id IS NOT NULL;

-- Add comments for documentation
COMMENT ON TABLE project_members IS 'Tracks users who have access to projects with their roles';
COMMENT ON COLUMN project_members.role IS 'User role in project: owner (full control) or member (read/write access)';
COMMENT ON COLUMN conversations.created_by_user_id IS 'User who created this conversation';
COMMENT ON COLUMN conversations.visibility IS 'Conversation visibility: private (creator only) or public (all project members)';