-- Performance optimization indexes
-- Created: 2025-10-04
-- Purpose: Add missing indexes to improve query performance

-- Index for messages table: Composite index on (conversation_id, created_at)
-- Used in: conversation message queries, message listing
CREATE INDEX IF NOT EXISTS idx_messages_conversation_created
ON messages(conversation_id, created_at)
WHERE (is_forgotten = false OR is_forgotten IS NULL);

-- Index for tool_usages table: Foreign key index on message_id
-- Used in: JOIN queries between messages and tool_usages
CREATE INDEX IF NOT EXISTS idx_tool_usages_message_id
ON tool_usages(message_id);

-- Index for analysis_jobs table: Status filter index
-- Used in: job filtering, status-based queries
CREATE INDEX IF NOT EXISTS idx_analysis_jobs_status
ON analysis_jobs(status);

-- Composite index for analysis_jobs: analysis_id + created_at
-- Used in: analytics queries, job history
CREATE INDEX IF NOT EXISTS idx_analysis_jobs_analysis_created
ON analysis_jobs(analysis_id, created_at DESC);

-- Index for tool_usages table: Composite index for updated_at queries
-- Used in: Recent tool usage queries
CREATE INDEX IF NOT EXISTS idx_tool_usages_updated_at
ON tool_usages(updated_at)
WHERE updated_at IS NOT NULL;
