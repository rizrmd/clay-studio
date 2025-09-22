import React, { useState, useEffect, useRef } from "react";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Check, X, Edit } from "lucide-react";
import { cn } from "@/lib/utils";

interface EditableCellProps {
  value: any;
  dataType: string;
  onSave: (newValue: any) => void;
  onCancel?: () => void;
  disabled?: boolean;
  className?: string;
  multiline?: boolean;
}

export function EditableCell({
  value,
  dataType,
  onSave,
  onCancel,
  disabled = false,
  className,
  multiline = false,
}: EditableCellProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(String(value ?? ""));
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement>(null);

  useEffect(() => {
    setEditValue(String(value ?? ""));
  }, [value]);

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      if (inputRef.current instanceof HTMLInputElement) {
        inputRef.current.select();
      }
    }
  }, [isEditing]);

  const handleSave = () => {
    let processedValue: any = editValue;

    // Type conversion based on data type
    switch (dataType) {
      case "number":
        const numValue = parseFloat(editValue);
        processedValue = isNaN(numValue) ? value : numValue;
        break;
      case "boolean":
        processedValue = editValue.toLowerCase() === "true";
        break;
      case "date":
        // Validate date format
        const dateValue = new Date(editValue);
        if (isNaN(dateValue.getTime())) {
          processedValue = value; // Keep original if invalid
        } else {
          processedValue = editValue;
        }
        break;
      default:
        processedValue = editValue;
    }

    onSave(processedValue);
    setIsEditing(false);
  };

  const handleCancel = () => {
    setEditValue(String(value ?? ""));
    setIsEditing(false);
    onCancel?.();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !multiline) {
      e.preventDefault();
      handleSave();
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCancel();
    }
  };

  const formatDisplayValue = (val: any) => {
    if (val === null || val === undefined) return "";
    
    switch (dataType) {
      case "date":
        try {
          return new Date(val).toLocaleDateString();
        } catch {
          return String(val);
        }
      case "boolean":
        return String(val);
      case "number":
        return typeof val === "number" ? val.toLocaleString() : String(val);
      default:
        return String(val);
    }
  };

  if (disabled) {
    return (
      <div className={cn("py-1 px-2", className)}>
        {formatDisplayValue(value)}
      </div>
    );
  }

  if (isEditing) {
    const InputComponent = multiline ? Textarea : Input;

    return (
      <div className="flex items-center gap-1 w-full">
        <InputComponent
          ref={inputRef as any}
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 h-7 text-xs"
          type={dataType === "number" ? "number" : dataType === "date" ? "date" : "text"}
        />
        <div className="flex gap-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={handleSave}
            className="h-6 w-6 p-0"
          >
            <Check className="h-3 w-3 text-green-600" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleCancel}
            className="h-6 w-6 p-0"
          >
            <X className="h-3 w-3 text-red-600" />
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "group relative py-1 px-2 cursor-pointer hover:bg-muted/50 flex items-center justify-between w-full",
        className
      )}
      onClick={() => setIsEditing(true)}
    >
      <span className="flex-1 truncate">
        {formatDisplayValue(value)}
      </span>
      <Edit className="h-3 w-3 opacity-0 group-hover:opacity-50 transition-opacity" />
    </div>
  );
}