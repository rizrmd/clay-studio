import { useState, useEffect } from "react";
import { FileText, Save, RotateCcw, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { api } from "@/lib/utils/api";

interface ClaudeMdEditorProps {
  projectId: string;
  isCollapsed?: boolean;
}

export function ClaudeMdEditor({ projectId, isCollapsed }: ClaudeMdEditorProps) {
  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasChanges, setHasChanges] = useState(false);

  // Fetch CLAUDE.md content
  useEffect(() => {
    if (!projectId) return;

    const fetchContent = async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await api.fetchStream(`/projects/${projectId}/claude-md`);

        if (!response.ok) {
          if (response.status === 404) {
            // File doesn't exist yet, start with empty content
            const defaultContent = `# Project Context for Claude

## Project Overview
This is a Clay Studio project workspace.

## Available Tools
- Claude Code SDK integration
- File management
- Query execution

## Notes
Add any project-specific context or instructions here that Claude should be aware of.
`;
            setContent(defaultContent);
            setOriginalContent(defaultContent);
            return;
          }
          throw new Error("Failed to fetch CLAUDE.md");
        }

        const data = await response.json();
        setContent(data.content);
        setOriginalContent(data.content);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load CLAUDE.md");
      } finally {
        setLoading(false);
      }
    };

    fetchContent();
  }, [projectId]);

  // Check for changes
  useEffect(() => {
    setHasChanges(content !== originalContent);
  }, [content, originalContent]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      const response = await api.fetchStream(`/projects/${projectId}/claude-md`, {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ content }),
      });

      if (!response.ok) {
        throw new Error("Failed to save CLAUDE.md");
      }

      setOriginalContent(content);
      setHasChanges(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save CLAUDE.md");
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    setContent(originalContent);
    setHasChanges(false);
  };

  if (isCollapsed) {
    return (
      <div className="p-2 border-t border-gray-200 dark:border-gray-700">
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-center"
          title="CLAUDE.md"
        >
          <FileText className="h-4 w-4" />
        </Button>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="p-4 border-t border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-gray-400" />
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 border-t border-gray-200 dark:border-gray-700">
      <Card className="border-0 shadow-none bg-transparent">
        <CardHeader className="p-0 pb-3">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <FileText className="h-4 w-4" />
            CLAUDE.md
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0 space-y-3">
          {error && (
            <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/20 p-2 rounded">
              {error}
            </div>
          )}
          
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="Add project-specific context for Claude..."
            className="min-h-[200px] text-xs font-mono bg-gray-50 dark:bg-gray-900/50"
            rows={8}
          />
          
          <div className="flex gap-2">
            <Button
              onClick={handleSave}
              disabled={saving || !hasChanges}
              size="sm"
              className="flex-1"
            >
              {saving ? (
                <>
                  <Loader2 className="h-3 w-3 animate-spin mr-1" />
                  Saving...
                </>
              ) : (
                <>
                  <Save className="h-3 w-3 mr-1" />
                  Save
                </>
              )}
            </Button>
            
            <Button
              onClick={handleReset}
              disabled={!hasChanges}
              variant="outline"
              size="sm"
            >
              <RotateCcw className="h-3 w-3" />
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}