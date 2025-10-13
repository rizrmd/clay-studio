import { ChevronLeft, X, MessageSquare, Trash2, Code, Users, Settings } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { sidebarActions, sidebarStore } from "@/lib/store/chat/sidebar-store";
import { tabsActions } from "@/lib/store/tabs-store";

interface ConversationSidebarHeaderProps {
  onNavigateToProjects: () => void;
  onBulkDelete: () => void;
  onBulkDeleteAnalyses?: () => void;
  projectId?: string;
  currentUserId?: string;
}

export function ConversationSidebarHeader({
  onNavigateToProjects,
  onBulkDelete,
  onBulkDeleteAnalyses,
  projectId,
}: ConversationSidebarHeaderProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const navigate = useNavigate();

  const isInDeleteMode = sidebarSnapshot.isDeleteMode || sidebarSnapshot.isAnalysisDeleteMode;

  return (
    <div className="px-1 py-2 border-b">
      <div className="flex items-center justify-between">
        {!isInDeleteMode && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              onNavigateToProjects();
              navigate('/projects');
            }}
            className="pl-1 gap-1 h-[25px] border border-transparent hover:border-gray-200"
          >
            <ChevronLeft size={10} />
            <span className="text-xs">Projects</span>
          </Button>
        )}

        {sidebarSnapshot.isDeleteMode ? (
          <div className="flex items-center justify-center flex-1 gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => sidebarActions.exitDeleteMode()}
              className="gap-1 h-[25px] border border-transparent hover:border-gray-200"
              title="Cancel"
            >
              <X size={10} />
              <span className="text-xs">Cancel</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={onBulkDelete}
              className="gap-1 h-[25px] border border-transparent hover:border-red-500 hover:text-red-600"
              title="Delete Selected"
            >
              <Trash2 size={10} />
              <span className="text-xs">
                Delete ({sidebarSnapshot.selectedConversations.length})
              </span>
            </Button>
          </div>
        ) : sidebarSnapshot.isAnalysisDeleteMode ? (
          <div className="flex items-center justify-center flex-1 gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => sidebarActions.exitAnalysisDeleteMode()}
              className="gap-1 h-[25px] border border-transparent hover:border-gray-200"
              title="Cancel"
            >
              <X size={10} />
              <span className="text-xs">Cancel</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={onBulkDeleteAnalyses}
              className="gap-1 h-[25px] border border-transparent hover:border-red-500 hover:text-red-600"
              title="Delete Selected"
            >
              <Trash2 size={10} />
              <span className="text-xs">
                Delete ({sidebarSnapshot.selectedAnalyses.length})
              </span>
            </Button>
          </div>
        ) : (
          <div className="flex items-center gap-1">
            {projectId && (
              <>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="gap-1 h-[25px] border border-transparent hover:border-gray-200"
                      title="Project Settings"
                    >
                      <Settings size={10} />
                      <span className="text-xs">Project</span>
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onClick={() => {
                        // Open members in a new tab
                        tabsActions.openInNewTab('members', {
                          projectId,
                        }, 'Members');
                        // Navigate to members route
                        navigate(`/p/${projectId}/members`);
                      }}
                    >
                      <Users className="h-4 w-4 mr-2" />
                      Members
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => {
                        // Open context in a new tab
                        tabsActions.openInNewTab('context', {
                          projectId,
                        }, 'Context');
                        // Navigate to context route
                        navigate(`/p/${projectId}/context`);
                      }}
                    >
                      <Code className="h-4 w-4 mr-2" />
                      Context
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                if (projectId) {
                  // Check for existing "New Chat" tab first, create only if needed
                  tabsActions.getOrCreateActiveTab('chat', {
                    projectId,
                    conversationId: 'new',
                  }, 'New Chat');

                  // Navigate to the new route
                  navigate(`/p/${projectId}/new`);
                }
              }}
              className="gap-1 h-[25px] border border-transparent hover:border-gray-200"
              title="New Chat"
            >
              <MessageSquare size={10} />
              <span className="text-xs">New</span>
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
