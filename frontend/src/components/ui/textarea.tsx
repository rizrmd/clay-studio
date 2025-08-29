import * as React from "react";

import { cn } from "@/lib/utils";

interface TextareaProps extends React.ComponentProps<"textarea"> {
  onEnterSubmit?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  placeholderSecondary?: string;
}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  (
    { className, onEnterSubmit, onKeyDown, placeholderSecondary, ...props },
    ref
  ) => {
    const [hasContent, setHasContent] = React.useState(!!props.value || !!props.defaultValue);
    
    const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter") {
        if (e.altKey || e.ctrlKey || e.metaKey) {
          // Alt/Ctrl/Cmd+Enter: Insert new line manually
          e.preventDefault();
          const target = e.target as HTMLTextAreaElement;
          const start = target.selectionStart;
          const end = target.selectionEnd;
          const value = target.value;

          // Insert new line at cursor position
          const newValue =
            value.substring(0, start) + "\n" + value.substring(end);

          // Trigger onChange manually since we're preventing default
          const syntheticEvent = {
            ...e,
            target: { ...target, value: newValue },
            currentTarget: target,
          } as React.ChangeEvent<HTMLTextAreaElement>;

          // Call onChange if it exists in props
          if (props.onChange) {
            props.onChange(syntheticEvent);
          }

          // Set cursor position after the new line
          setTimeout(() => {
            target.selectionStart = target.selectionEnd = start + 1;
          }, 0);
        } else if (!e.shiftKey) {
          // Plain Enter: Submit if handler exists
          if (onEnterSubmit) {
            e.preventDefault();
            onEnterSubmit(e);
          }
        }
        // Shift+Enter: Let default behavior handle it (new line)
      }

      // Call the original onKeyDown if provided
      onKeyDown?.(e);
    };

    const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setHasContent(!!e.target.value);
      props.onChange?.(e);
    };

    const main = (
      <textarea
        className={cn(
          "flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-base shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
          className
        )}
        ref={ref}
        onKeyDown={handleKeyDown}
        {...props}
        onChange={handleChange}
        placeholder=""
      />
    );

    if (!placeholderSecondary) {
      return main;
    }

    // Update hasContent when value prop changes
    React.useEffect(() => {
      if (props.value !== undefined) {
        setHasContent(!!props.value);
      }
    }, [props.value]);

    return (
      <div className="relative">
        {main}
        {placeholderSecondary && !hasContent && (
          <div className="absolute inset-0 px-3 -mt-1 pointer-events-none flex flex-col justify-center">
            <div className="text-muted-foreground text-base md:text-sm">
              {props.placeholder}
            </div>
            <div className="text-muted-foreground/70 text-xs mt-0.5">
              {placeholderSecondary}
            </div>
          </div>
        )}
      </div>
    );
  }
);
Textarea.displayName = "Textarea";

export { Textarea };
