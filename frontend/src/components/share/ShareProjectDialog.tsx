import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Copy,
  Share2,
  ExternalLink,
} from "lucide-react";
import { useChat } from "@/lib/hooks/use-chat";
import { sharesApi, CreateShareRequest, ShareResponse, ShareType, ShareSettings } from "@/lib/api/shares";
import { toast } from "sonner";

interface ShareProjectDialogProps {
  isOpen: boolean;
  onClose: () => void;
  projectId: string;
  preSelectedConversations?: string[]; // Pre-select specific conversations
}


export function ShareProjectDialog({ isOpen, onClose, projectId, preSelectedConversations }: ShareProjectDialogProps) {
  const [shareType, setShareType] = useState<ShareType>("all_history");
  const [settings, setSettings] = useState<ShareSettings>({
    theme: "light",
    show_branding: true,
    allow_file_upload: true,
    show_conversation_list: true,
    show_project_name: true,
    enable_markdown: true,
    layout_mode: "full",
  });
  const [isReadOnly, setIsReadOnly] = useState(false);
  const [selectedConversations, setSelectedConversations] = useState<string[]>([]);
  const [isCreating, setIsCreating] = useState(false);
  const [shareResult, setShareResult] = useState<ShareResponse | null>(null);

  const { conversationList, conversationMap } = useChat();

  // Pre-configure dialog for specific conversation sharing
  useEffect(() => {
    if (preSelectedConversations && preSelectedConversations.length > 0) {
      setShareType("specific_conversations");
      setSelectedConversations(preSelectedConversations);
      // Update the dialog title/description if needed
      const conversation = conversationMap[preSelectedConversations[0]];
      if (conversation && preSelectedConversations.length === 1) {
        setSettings(prev => ({
          ...prev,
          title: `Share: ${conversation.title || 'Conversation'}`,
          description: `Shared conversation from ${new Date().toLocaleDateString()}`
        }));
      }
    }
  }, [preSelectedConversations, conversationMap]);

  const handleCreateShare = async () => {
    setIsCreating(true);
    try {
      const request: CreateShareRequest = {
        share_type: shareType,
        settings,
        is_read_only: isReadOnly,
        conversation_ids: shareType === "specific_conversations" ? selectedConversations : undefined,
      };

      const response = await sharesApi.createShare(projectId, request);
      setShareResult(response);
      toast.success("Share link created successfully!");
    } catch (error: any) {
      console.error("Error creating share:", error);
      const errorMessage = error?.response?.data?.message || error?.message || "Failed to create share link";
      toast.error(errorMessage);
    } finally {
      setIsCreating(false);
    }
  };

  const copyToClipboard = async (text: string, type: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`${type} copied to clipboard!`);
    } catch (error) {
      toast.error("Failed to copy to clipboard");
    }
  };

  const resetDialog = () => {
    setShareResult(null);
    setShareType("all_history");
    setSelectedConversations([]);
    setSettings({
      theme: "light",
      show_branding: true,
      allow_file_upload: true,
      show_conversation_list: true,
      show_project_name: true,
      enable_markdown: true,
      layout_mode: "full",
    });
    setIsReadOnly(false);
  };

  const handleClose = () => {
    resetDialog();
    onClose();
  };

  if (shareResult) {
    return (
      <Dialog open={isOpen} onOpenChange={handleClose}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Share2 className="w-5 h-5" />
              Share Link Created
            </DialogTitle>
            <DialogDescription>
              Your shareable link is ready. Copy and share it with others.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            {/* Share URL */}
            <div className="space-y-2">
              <Label>Share URL</Label>
              <div className="flex gap-2">
                <Input 
                  value={shareResult.embed_url} 
                  readOnly 
                  className="font-mono text-sm"
                  onClick={(e) => e.currentTarget.select()}
                />
                <Button
                  size="icon"
                  variant="outline"
                  onClick={() => copyToClipboard(shareResult.embed_url, "Link")}
                >
                  <Copy className="h-4 w-4" />
                </Button>
                <Button
                  size="icon"
                  variant="outline"
                  onClick={() => window.open(shareResult.embed_url, "_blank")}
                >
                  <ExternalLink className="h-4 w-4" />
                </Button>
              </div>
            </div>

            {/* Simple embed code */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>Embed Code</Label>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => copyToClipboard(shareResult.embed_codes.iframe_simple, "Embed code")}
                >
                  <Copy className="h-3 w-3 mr-1" />
                  Copy
                </Button>
              </div>
              <div className="rounded-md border bg-muted p-2">
                <code className="text-xs break-all">
                  {shareResult.embed_codes.iframe_simple}
                </code>
              </div>
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <Button variant="outline" onClick={() => setShareResult(null)}>
                Create Another
              </Button>
              <Button onClick={handleClose}>Done</Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Share2 className="w-5 h-5" />
            Share Project
          </DialogTitle>
          <DialogDescription>
            Create a shareable link for your Clay Studio project
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Share Type Selection */}
          <div className="space-y-2">
            <Label>Share Type</Label>
            <Select value={shareType} onValueChange={(value: ShareType) => setShareType(value)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all_history">All Conversations</SelectItem>
                <SelectItem value="specific_conversations">Selected Conversations</SelectItem>
                <SelectItem value="new_chat">New Chat Only</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Conversation Selection */}
          {shareType === "specific_conversations" && (
            <div className="space-y-2">
              <Label>Select Conversations</Label>
              <ScrollArea className="h-32 border rounded-md p-2">
                {conversationList.map((convId) => {
                  const conversation = conversationMap[convId];
                  if (!conversation) return null;
                  
                  return (
                    <div key={convId} className="flex items-center space-x-2 py-1">
                      <input
                        type="checkbox"
                        id={convId}
                        checked={selectedConversations.includes(convId)}
                        onChange={(e) => {
                          if (e.target.checked) {
                            setSelectedConversations([...selectedConversations, convId]);
                          } else {
                            setSelectedConversations(
                              selectedConversations.filter((id) => id !== convId)
                            );
                          }
                        }}
                        className="rounded"
                      />
                      <Label htmlFor={convId} className="text-sm cursor-pointer flex-1">
                        {conversation.title || `Untitled Chat`}
                      </Label>
                    </div>
                  );
                })}
              </ScrollArea>
              {selectedConversations.length > 0 && (
                <p className="text-xs text-muted-foreground">
                  {selectedConversations.length} selected
                </p>
              )}
            </div>
          )}

          {/* Access Settings */}
          <div className="flex items-center justify-between py-2">
            <div>
              <Label>Read-only mode</Label>
              <p className="text-sm text-muted-foreground">
                Viewers can only read, not interact
              </p>
            </div>
            <Switch checked={isReadOnly} onCheckedChange={setIsReadOnly} />
          </div>

        </div>

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="outline" onClick={handleClose}>
            Cancel
          </Button>
          <Button 
            onClick={handleCreateShare} 
            disabled={isCreating || (shareType === "specific_conversations" && selectedConversations.length === 0)}
          >
            {isCreating ? "Creating..." : "Create Share Link"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}