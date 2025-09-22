import { useState, useEffect, useRef } from "react";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Check, X, Ban } from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

interface EditPopupProps {
  value: any;
  dataType: string;
  nullable?: boolean;
  onSave: (newValue: any) => void;
  onCancel?: () => void;
  disabled?: boolean;
  children: React.ReactNode;
  multiline?: boolean;
}

export function EditPopup({
  value,
  dataType,
  nullable = false,
  onSave,
  onCancel,
  disabled = false,
  children,
  multiline = false,
}: EditPopupProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [editValue, setEditValue] = useState(String(value ?? ""));
  const [isNull, setIsNull] = useState(value === null || value === undefined);
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement>(null);

  useEffect(() => {
    setEditValue(String(value ?? ""));
    setIsNull(value === null || value === undefined);
  }, [value]);

  useEffect(() => {
    if (isOpen && inputRef.current) {
      // Focus and select all text after a brief delay to ensure popover is rendered
      setTimeout(() => {
        inputRef.current?.focus();
        if (inputRef.current instanceof HTMLInputElement) {
          inputRef.current.select();
        }
      }, 100);
    }
  }, [isOpen]);

  const handleSave = () => {
    // If user selected NULL, save null value
    if (isNull) {
      onSave(null);
      setIsOpen(false);
      return;
    }

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
    setIsOpen(false);
  };

  const handleSetNull = () => {
    setIsNull(true);
    setEditValue("");
  };

  const handleClearNull = () => {
    setIsNull(false);
    setEditValue(String(value ?? ""));
  };

  const handleCancel = () => {
    setEditValue(String(value ?? ""));
    setIsNull(value === null || value === undefined);
    setIsOpen(false);
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

  const handleOpenChange = (open: boolean) => {
    if (!disabled) {
      setIsOpen(open);
      if (!open) {
        // Reset value when closing without saving
        setEditValue(String(value ?? ""));
        setIsNull(value === null || value === undefined);
      }
    }
  };

  if (disabled) {
    return <>{children}</>;
  }

  const InputComponent = multiline ? Textarea : Input;

  return (
    <Popover open={isOpen} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        <div className="cursor-pointer hover:bg-muted/50 -m-1 py-1 rounded">
          {children}
        </div>
      </PopoverTrigger>
      <PopoverContent className="w-80" align="start">
        <div className="space-y-3">
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm font-medium">Edit Value</label>
              {nullable && (
                <div className="flex items-center gap-2">
                  {isNull ? (
                    <Badge variant="secondary" className="text-xs">
                      <Ban className="h-3 w-3 mr-1" />
                      NULL
                    </Badge>
                  ) : (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={handleSetNull}
                      className="h-6 px-2 text-xs"
                    >
                      <Ban className="h-3 w-3 mr-1" />
                      Set NULL
                    </Button>
                  )}
                </div>
              )}
            </div>

            {isNull ? (
              <div className="flex items-center justify-center h-9 border-2 border-dashed border-muted-foreground/25 rounded-md bg-muted/20">
                <span className="text-sm text-muted-foreground">
                  Value is NULL
                </span>
              </div>
            ) : (
              <InputComponent
                ref={inputRef as any}
                value={editValue}
                onChange={(e) => {
                  setEditValue(e.target.value);
                  setIsNull(false);
                }}
                onKeyDown={handleKeyDown}
                className="mt-1"
                type={
                  dataType === "number"
                    ? "number"
                    : dataType === "date"
                    ? "date"
                    : "text"
                }
                placeholder={`Enter ${dataType} value...`}
              />
            )}
          </div>

          <div className="flex justify-between items-center">
            <div className="flex gap-1">
              {nullable && isNull && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={handleClearNull}
                  className="h-7 px-2 text-xs"
                >
                  Clear NULL
                </Button>
              )}
            </div>
            <div className="flex gap-2">
              <Button size="sm" variant="outline" onClick={handleCancel}>
                <X className="h-4 w-4 mr-1" />
                Cancel
              </Button>
              <Button size="sm" onClick={handleSave}>
                <Check className="h-4 w-4 mr-1" />
                Save
              </Button>
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
