"use client";

import React, { useState, useRef, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { MoreHorizontal } from "lucide-react";
import { cn } from "@/lib/utils";

interface CellContentProps {
  content: React.ReactNode;
  columnKey?: string;
  rowId?: string | number;
  maxHeight?: number;
}

export function CellContent({
  content,
  columnKey = "cell",
  rowId: _rowId = "row",
  maxHeight = 50,
}: CellContentProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [showButton, setShowButton] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (contentRef.current) {
      // Check if content exceeds max height
      const contentHeight = contentRef.current.scrollHeight;
      setShowButton(contentHeight > maxHeight);
    }
  }, [content, maxHeight]);

  // Safe conversion to string for display in dialog
  const getContentString = () => {
    try {
      if (content === null || content === undefined) {
        return "";
      }
      if (typeof content === "string" || typeof content === "number" || typeof content === "boolean") {
        return String(content);
      }
      if (React.isValidElement(content)) {
        return "[React Component]";
      }
      if (content instanceof HTMLElement) {
        return "[HTML Element]";
      }
      if (typeof content === "object") {
        // Try to stringify, but catch circular reference errors
        try {
          return JSON.stringify(content, null, 2);
        } catch {
          return "[Complex Object]";
        }
      }
      return String(content);
    } catch {
      return "[Unable to display]";
    }
  };
  
  const contentString = getContentString();

  return (
    <>
      <div className="relative">
        <div
          ref={contentRef}
          className={cn(
            "overflow-hidden",
            showButton && "line-clamp-2"
          )}
          style={{
            maxHeight: showButton ? `${maxHeight}px` : undefined,
          }}
        >
          {content}
        </div>
        {showButton && (
          <Button
            variant="ghost"
            size="sm"
            className="absolute -bottom-1 right-0 h-6 px-1 text-xs bg-background/90 hover:bg-background"
            onClick={() => setIsOpen(true)}
          >
            <MoreHorizontal className="h-3 w-3 mr-1" />
            More
          </Button>
        )}
      </div>

      <Dialog open={isOpen} onOpenChange={setIsOpen}>
        <DialogContent className="max-w-3xl max-h-[80vh] overflow-auto">
          <DialogHeader>
            <DialogTitle>
              Full Content - {columnKey}
            </DialogTitle>
          </DialogHeader>
          <div className="mt-4">
            {typeof content === "object" ? (
              <pre className="text-sm font-mono whitespace-pre-wrap bg-muted p-4 rounded-lg overflow-auto">
                {contentString}
              </pre>
            ) : (
              <div className="text-sm whitespace-pre-wrap break-words">
                {content}
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}