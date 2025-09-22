-- Add tool_use_id column to tool_usages table to properly track tool calls from Claude SDK
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tool_usages' AND column_name = 'tool_use_id') THEN
        ALTER TABLE tool_usages ADD COLUMN tool_use_id VARCHAR(255);
    END IF;
END $$;

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_tool_usages_tool_use_id ON tool_usages(tool_use_id);

-- Add comment explaining the column
COMMENT ON COLUMN tool_usages.tool_use_id IS 'Unique identifier from Claude SDK for matching tool calls with results';