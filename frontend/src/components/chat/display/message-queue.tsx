import { useState } from "react";
import { Clock, Edit2, X, Check, MessageSquare } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import type { QueuedMessage } from "@/hooks/use-valtio-chat";

interface MessageQueueProps {
  messageQueue: QueuedMessage[];
  editQueuedMessage: (messageId: string, newContent: string) => void;
  removeQueuedMessage: (messageId: string) => void;
  isProcessing: boolean;
}

export function MessageQueue({
  messageQueue,
  editQueuedMessage,
  removeQueuedMessage,
  isProcessing
}: MessageQueueProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  if (messageQueue.length === 0) {
    return null;
  }

  const handleStartEdit = (id: string, content: string) => {
    setEditingId(id);
    setEditValue(content);
  };

  const handleSaveEdit = () => {
    if (editingId !== null) {
      editQueuedMessage(editingId, editValue);
      setEditingId(null);
      setEditValue("");
    }
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setEditValue("");
  };

  return (
    <div className="mx-auto max-w-2xl px-4 mb-4">
      <div className="bg-amber-50 border border-amber-200 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <MessageSquare className="h-4 w-4 text-amber-600" />
            <span className="text-sm font-medium text-amber-900">
              Queued Messages
            </span>
            <Badge variant="secondary" className="text-xs">
              {messageQueue.length}
            </Badge>
            {isProcessing && (
              <Badge variant="outline" className="text-xs">
                <Clock className="h-3 w-3 mr-1" />
                Processing...
              </Badge>
            )}
          </div>
        </div>

        <div className="space-y-2">
          {messageQueue.map((message, index) => (
            <div
              key={message.id}
              className={cn(
                "bg-white border rounded p-3 transition-all",
                index === 0 && isProcessing && "border-green-300 bg-green-50",
                !message.isEditable && "border-blue-300 bg-blue-50",
                message.isEditable && index > 0 && "border-gray-200"
              )}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-xs font-medium text-gray-500">
                      #{index + 1}
                    </span>
                    {!message.isEditable && (
                      <Badge variant="outline" className="text-xs">
                        <Clock className="h-3 w-3 mr-1 animate-spin" />
                        Processing
                      </Badge>
                    )}
                    {message.files && message.files.length > 0 && (
                      <Badge variant="secondary" className="text-xs">
                        {message.files.length} file{message.files.length > 1 ? 's' : ''}
                      </Badge>
                    )}
                    <span className="text-xs text-gray-400">
                      {new Date(message.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  
                  {editingId === message.id ? (
                    <div className="space-y-2">
                      <Textarea
                        value={editValue}
                        onChange={(e) => setEditValue(e.target.value)}
                        className="min-h-[80px] text-sm"
                        placeholder="Edit your message..."
                        autoFocus
                      />
                      <div className="flex gap-1">
                        <Button
                          size="sm"
                          onClick={handleSaveEdit}
                          className="h-7 px-2 text-xs"
                        >
                          <Check className="h-3 w-3 mr-1" />
                          Save
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={handleCancelEdit}
                          className="h-7 px-2 text-xs"
                        >
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <div className="text-sm text-gray-700 whitespace-pre-wrap break-words">
                      {message.content}
                    </div>
                  )}
                </div>
                
                {editingId !== message.id && message.isEditable && (
                  <div className="flex gap-1 flex-shrink-0">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleStartEdit(message.id, message.content)}
                      className="h-7 w-7 p-0 text-gray-500 hover:text-gray-700"
                    >
                      <Edit2 className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => removeQueuedMessage(message.id)}
                      className="h-7 w-7 p-0 text-gray-500 hover:text-red-600"
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
        
        <div className="mt-3 text-xs text-amber-700">
          Messages will be sent one at a time in order. You can edit or remove queued messages before they're processed.
        </div>
      </div>
    </div>
  );
}