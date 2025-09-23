import React from "react";
import { Ban, CheckCircle } from "lucide-react";
import { ReloadIcon, RocketIcon } from "@radix-ui/react-icons";

interface PartialToolCall {
  state: "partial-call";
  toolName: string;
}

interface ToolCall {
  state: "call";
  toolName: string;
}

interface ToolResult {
  state: "result";
  toolName: string;
  result: {
    __cancelled?: boolean;
    __completed?: boolean;
    executionTime?: number;
    [key: string]: any;
  };
}

type ToolInvocation = (PartialToolCall | ToolCall | ToolResult) & {
  id: string;
};

interface ChatProgressProps {
  toolInvocations?: ToolInvocation[];
}

export const ChatProgress: React.FC<ChatProgressProps> = ({
  toolInvocations,
}) => {
  if (!toolInvocations?.length) return null;

  const progressInvocations = toolInvocations.filter(
    (invocation) =>
      invocation.state === "partial-call" ||
      invocation.state === "call" ||
      (invocation.state === "result" && invocation.result.__cancelled === true) ||
      (invocation.state === "result" && invocation.result.__completed === true)
  );

  if (!progressInvocations.length) return null;

  return (
    <div className="flex text-xs flex-row gap-2 flex-wrap">
      {progressInvocations.map((invocation, index) => {
        const isCancelled =
          invocation.state === "result" &&
          invocation.result.__cancelled === true;

        const isCompleted =
          invocation.state === "result" &&
          invocation.result.__completed === true;

        if (isCancelled) {
          return (
            <div
              key={index}
              className="flex items-center gap-2 border tool-use text-muted-foreground"
            >
              <Ban />
              <span>
                Cancelled{" "}
                <span className="font-mono">
                  {"`"}
                  {invocation.toolName}
                  {"`"}
                </span>
              </span>
            </div>
          );
        }

        if (isCompleted) {
          return (
            <div
              key={index}
              className="flex items-center gap-2 border tool-use text-green-600"
            >
              <CheckCircle className="h-3 w-3" />
              <span>
                Completed{" "}
                <span className="font-mono">
                  {"`"}
                  {invocation.toolName
                    .split("__")
                    .pop()
                    ?.split("_")
                    .join(" ")}
                  {"`"}
                </span>
                {invocation.result?.executionTime && (
                  <span className="text-muted-foreground ml-1">
                    ({invocation.result.executionTime}ms)
                  </span>
                )}
              </span>
            </div>
          );
        }

        switch (invocation.state) {
          case "partial-call":
          case "call":
            return (
              <div
                key={index}
                className="flex items-center gap-2 border tool-use text-muted-foreground"
              >
                <RocketIcon />
                <span>
                  Calling{" "}
                  <span className="font-mono">
                    {invocation.toolName
                      .split("__")
                      .pop()
                      ?.split("_")
                      .join(" ")}
                  </span>
                  ...
                </span>
                <ReloadIcon className="h-3 w-3 animate-spin" />
              </div>
            );
          default:
            return null;
        }
      })}
    </div>
  );
};