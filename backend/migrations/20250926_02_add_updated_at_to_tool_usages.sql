-- Add updated_at column to tool_usages table for tracking last modification time
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tool_usages' AND column_name = 'updated_at') THEN
        ALTER TABLE tool_usages ADD COLUMN updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;
    END IF;
END $$;

-- Create trigger to automatically update the updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Drop trigger if exists and create new one
DROP TRIGGER IF EXISTS update_tool_usages_updated_at ON tool_usages;
CREATE TRIGGER update_tool_usages_updated_at 
    BEFORE UPDATE ON tool_usages 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Add comment explaining the column
COMMENT ON COLUMN tool_usages.updated_at IS 'Timestamp of last modification, automatically updated on changes';

-- Update existing rows to have updated_at same as created_at if it exists
UPDATE tool_usages 
SET updated_at = COALESCE(created_at, CURRENT_TIMESTAMP) 
WHERE updated_at IS NULL;