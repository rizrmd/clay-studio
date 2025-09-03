import { useState, useEffect } from "react";
import { FileText, Save, RotateCcw, Loader2, X, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api } from "@/lib/utils/api";

interface ClaudeMdModalProps {
  projectId: string;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ClaudeMdModal({ projectId, isOpen, onOpenChange }: ClaudeMdModalProps) {
  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasChanges, setHasChanges] = useState(false);

  // Fetch CLAUDE.md content when modal opens
  useEffect(() => {
    if (!projectId || !isOpen) return;

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
  }, [projectId, isOpen]);

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

  const handleRefresh = async () => {
    setRefreshing(true);
    setError(null);
    try {
      const response = await api.fetchStream(`/projects/${projectId}/claude-md`, {
        method: "POST",
      });

      if (!response.ok) {
        throw new Error("Failed to refresh CLAUDE.md");
      }

      const data = await response.json();
      setContent(data.content);
      setOriginalContent(data.content);
      setHasChanges(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to refresh CLAUDE.md");
    } finally {
      setRefreshing(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="min-w-[90vw] max-w-[90vw] min-h-[80vh] max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            CLAUDE.md Editor
          </DialogTitle>
        </DialogHeader>
        
        <div className="flex-1 flex flex-col space-y-4">
          {error && (
            <div className="text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/20 p-3 rounded">
              {error}
            </div>
          )}
          
          {loading ? (
            <div className="flex-1 flex items-center justify-center">
              <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
            </div>
          ) : (
            <>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="Add project-specific context for Claude..."
                className="flex-1 min-h-0 text-sm font-mono bg-gray-50 dark:bg-gray-900/50"
              />
              
              <div className="flex gap-2">
                <Button
                  onClick={handleSave}
                  disabled={saving || !hasChanges}
                  size="sm"
                >
                  {saving ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin mr-2" />
                      Saving...
                    </>
                  ) : (
                    <>
                      <Save className="h-4 w-4 mr-2" />
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
                  <RotateCcw className="h-4 w-4 mr-2" />
                  Reset
                </Button>

                <Button
                  onClick={handleRefresh}
                  disabled={refreshing}
                  variant="outline"
                  size="sm"
                  title="Refresh CLAUDE.md with latest datasource information"
                >
                  {refreshing ? (
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  ) : (
                    <RefreshCw className="h-4 w-4 mr-2" />
                  )}
                  Refresh
                </Button>

                <div className="flex-1" />

                <Button
                  onClick={() => onOpenChange(false)}
                  variant="outline"
                  size="sm"
                >
                  <X className="h-4 w-4 mr-2" />
                  Close
                </Button>
              </div>
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}