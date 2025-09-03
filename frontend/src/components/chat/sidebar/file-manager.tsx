import { useState, useEffect } from 'react';
import { FileText, Edit2, Save, X, Loader2, Info } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { api } from '@/lib/utils/api';

interface FileUpload {
  id: string;
  file_name: string;
  original_name: string;
  file_path: string;
  size: number;
  mime_type?: string;
  description?: string;
  auto_description?: string;
  created_at: string;
  is_text_file: boolean;
  preview?: string;
}

interface FileManagerProps {
  projectId: string;
  conversationId?: string;
  onFileSelect?: (file: FileUpload) => void;
}

export function FileManager({ projectId, conversationId, onFileSelect }: FileManagerProps) {
  const [files, setFiles] = useState<FileUpload[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingFile, setEditingFile] = useState<string | null>(null);
  const [editDescription, setEditDescription] = useState('');
  const [savingDescription, setSavingDescription] = useState(false);

  useEffect(() => {
    fetchFiles();
  }, [projectId, conversationId]);

  const fetchFiles = async () => {
    try {
      const clientId = localStorage.getItem('activeClientId');
      if (!clientId) return;

      const params = new URLSearchParams({
        client_id: clientId,
        project_id: projectId,
      });
      
      if (conversationId) {
        params.append('conversation_id', conversationId);
      }

      const response = await api.fetchStream(`/uploads?${params}`);

      if (response.ok) {
        const data = await response.json();
        setFiles(data);
      }
    } catch (error) {
      // Failed to fetch files
    } finally {
      setLoading(false);
    }
  };

  const startEditDescription = (file: FileUpload) => {
    setEditingFile(file.id);
    setEditDescription(file.description || file.auto_description || '');
  };

  const cancelEdit = () => {
    setEditingFile(null);
    setEditDescription('');
  };

  const saveDescription = async (fileId: string) => {
    setSavingDescription(true);
    try {
      const response = await api.fetchStream(`/uploads/${fileId}/description`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          description: editDescription,
        }),
      });

      if (response.ok) {
        const updatedFile = await response.json();
        setFiles(files.map(f => f.id === fileId ? updatedFile : f));
        setEditingFile(null);
        setEditDescription('');
      }
    } catch (error) {
      // Failed to update description
    } finally {
      setSavingDescription(false);
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="h-6 w-6 animate-spin" />
      </div>
    );
  }

  if (files.length === 0) {
    return (
      <div className="text-center p-8 text-muted-foreground">
        No files uploaded yet
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="text-sm text-muted-foreground">
        {files.length} file{files.length !== 1 ? 's' : ''} uploaded
      </div>
      
      <div className="grid gap-3">
        {files.map((file) => (
          <Card key={file.id} className="hover:shadow-md transition-shadow">
            <CardHeader className="pb-3">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-2">
                  <FileText className="h-4 w-4 text-muted-foreground" />
                  <CardTitle className="text-sm font-medium">
                    {file.original_name}
                  </CardTitle>
                </div>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <span>{formatFileSize(file.size)}</span>
                  {file.mime_type && (
                    <span className="px-2 py-0.5 bg-muted rounded">
                      {file.mime_type.split('/')[1] || file.mime_type}
                    </span>
                  )}
                </div>
              </div>
            </CardHeader>
            
            <CardContent className="pt-0">
              {editingFile === file.id ? (
                <div className="flex gap-2">
                  <Input
                    value={editDescription}
                    onChange={(e) => setEditDescription(e.target.value)}
                    placeholder="Add a description..."
                    className="flex-1"
                    disabled={savingDescription}
                  />
                  <Button
                    size="icon"
                    variant="ghost"
                    onClick={() => saveDescription(file.id)}
                    disabled={savingDescription}
                  >
                    {savingDescription ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4" />
                    )}
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    onClick={cancelEdit}
                    disabled={savingDescription}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>
              ) : (
                <div className="space-y-2">
                  {(file.description || file.auto_description) && (
                    <div className="flex items-start gap-2">
                      <div className="flex-1 text-sm text-muted-foreground">
                        {file.description || file.auto_description}
                        {file.auto_description && !file.description && (
                          <TooltipProvider>
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Info className="inline-block h-3 w-3 ml-1" />
                              </TooltipTrigger>
                              <TooltipContent>
                                <p className="text-xs">Auto-generated description</p>
                              </TooltipContent>
                            </Tooltip>
                          </TooltipProvider>
                        )}
                      </div>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-6 w-6"
                        onClick={() => startEditDescription(file)}
                      >
                        <Edit2 className="h-3 w-3" />
                      </Button>
                    </div>
                  )}
                  
                  {!file.description && !file.auto_description && (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => startEditDescription(file)}
                      className="text-xs"
                    >
                      Add description
                    </Button>
                  )}
                  
                  {file.preview && (
                    <details className="text-xs">
                      <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
                        Preview content
                      </summary>
                      <pre className="mt-2 p-2 bg-muted rounded text-xs overflow-x-auto">
                        {file.preview}
                      </pre>
                    </details>
                  )}
                  
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span>{formatDate(file.created_at)}</span>
                    {onFileSelect && (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => onFileSelect(file)}
                        className="text-xs"
                      >
                        Use in chat
                      </Button>
                    )}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}