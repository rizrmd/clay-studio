import { Button } from "@/components/ui/button";
import { Bot, Square } from "lucide-react";
import { ToolCallIndicator } from "../tool/tool-call-indicator";

interface LoadingIndicatorProps {
  activeTools: string[];
  canStop: boolean;
  onStop?: () => void;
  thinkingWord: string;
}

export function LoadingIndicator({ 
  activeTools, 
  canStop, 
  onStop, 
  thinkingWord 
}: LoadingIndicatorProps) {
  return (
    <div className="flex max-w-[45rem] mx-auto cursor-default">
      <div className="flex flex-1 relative p-2 rounded-lg">
        <div className="flex gap-3 justify-start flex-1 pr-[45px]">
          <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
            <Bot className="h-4 w-4" />
          </div>
          <div className="flex flex-col gap-2 max-w-[70%]">
            <div className="rounded-lg p-3 text-sm bg-muted">
              <div className="flex items-center space-x-2">
                {activeTools.length === 0 ? (
                  <div className="flex items-center space-x-2 flex-1">
                    <div className="flex items-center space-x-1">
                      <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.3s]"></div>
                      <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.15s]"></div>
                      <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground"></div>
                    </div>
                    <span className="text-muted-foreground text-sm animate-pulse font-medium">
                      {thinkingWord}...
                    </span>
                  </div>
                ) : (
                  <span className="text-muted-foreground text-sm animate-pulse font-medium flex items-center justify-center flex-1">
                    {thinkingWord} {activeTools.length > 1 ? "tools" : "tool"}
                  </span>
                )}
                {canStop && onStop && (
                  <div className="pl-6">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={onStop}
                      className="h-7 px-2"
                    >
                      <Square className="h-3 w-3 mr-1" />
                      Stop
                    </Button>
                  </div>
                )}
              </div>
            </div>
            {activeTools.length > 0 && (
              <ToolCallIndicator
                key={`active-tools-${activeTools.join("-")}`}
                tools={activeTools}
                variant="full"
                isCompleted={false}
                className="ml-3"
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}