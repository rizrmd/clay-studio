import { ToolUsageDialog } from "@/components/chat/display/message/item/tools/tool-usage-dialog";
import { type ToolInvocation } from "@/components/chat/display/message/item/types";
import { cn } from "@/lib/utils";
import { Code2 } from "lucide-react";
import React, { useState } from "react";

interface CompletedToolCallProps {
  toolInvocations?: ToolInvocation[];
  className?: string;
}

export const ChatToolUsage: React.FC<CompletedToolCallProps> = ({
  toolInvocations,
  className,
}) => {
  const [expanded, setExpanded] = useState(false);

  if (!toolInvocations?.length) return null;

  const validInvocations = toolInvocations.filter(
    (invocation) =>
      invocation.state === "result" && !invocation.result.__cancelled
  );

  if (validInvocations.length === 0) return null;

  if (validInvocations.length === 1) {
    const invocation = validInvocations[0];

    return (
      <ToolUsageDialog toolUsageId={invocation.id}>
        <div
          className={cn(
            "flex flex-col gap-1.5 tool-use cursor-pointer ",
            className
          )}
        >
          <div className="flex items-center gap-2 text-muted-foreground px-1">
            <Code2 />
            <span>
              <span className="font-mono capitalize">
                {invocation.toolName.split("__").pop()?.split("_").join(" ")}
              </span>
            </span>
          </div>
        </div>
      </ToolUsageDialog>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <div
        className={cn(
          "flex flex-col gap-1.5 tool-use cursor-pointer ",
          className
        )}
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2 text-muted-foreground px-1">
          <Code2 />
          <span className="font-mono">
            {validInvocations.length} Tools used
          </span>
        </div>
      </div>

      {expanded && (
        <div className="flex flex-col gap-2 pl-4">
          {validInvocations.map((invocation, index) => {
            return (
              <ToolUsageDialog key={index} toolUsageId={invocation.id}>
                <div
                  className={cn(
                    "flex flex-col gap-1.5 tool-use cursor-pointer ",
                    className
                  )}
                >
                  <div className="flex items-center gap-2 text-muted-foreground px-1">
                    <Code2 />
                    <span>
                      <span className="font-mono capitalize">
                        {invocation.toolName
                          .split("__")
                          .pop()
                          ?.split("_")
                          .join(" ")}
                      </span>
                    </span>
                  </div>
                </div>
              </ToolUsageDialog>
            );
          })}
        </div>
      )}
    </div>
  );
};
