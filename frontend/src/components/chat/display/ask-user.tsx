import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card } from "@/components/ui/card";
import { Send, UserCheck } from "lucide-react";
import { cn } from "@/lib/utils";

interface AskUserOption {
  value: string;
  label: string;
  description?: string;
}

interface AskUserProps {
  promptType: "checkbox" | "buttons" | "input";
  title: string;
  options?: AskUserOption[];
  inputType?: "text" | "password";
  placeholder?: string;
  toolUseId?: string;
  onSubmit: (response: string | string[]) => void;
  isDisabled?: boolean;
}

export function AskUser({
  promptType,
  title,
  options = [],
  inputType = "text",
  placeholder,
  toolUseId,
  onSubmit,
  isDisabled = false,
}: AskUserProps) {
  const [selectedValues, setSelectedValues] = useState<string[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [hasSubmitted, setHasSubmitted] = useState(false);

  const handleCheckboxChange = (value: string, checked: boolean) => {
    if (checked) {
      setSelectedValues([...selectedValues, value]);
    } else {
      setSelectedValues(selectedValues.filter((v) => v !== value));
    }
  };

  const handleButtonClick = (value: string) => {
    if (hasSubmitted || isDisabled) return;
    setHasSubmitted(true);
    onSubmit(value);
  };

  const handleCheckboxSubmit = () => {
    if (hasSubmitted || isDisabled || selectedValues.length === 0) return;
    setHasSubmitted(true);
    onSubmit(selectedValues);
  };

  const handleInputSubmit = () => {
    if (hasSubmitted || isDisabled || !inputValue.trim()) return;
    setHasSubmitted(true);
    onSubmit(inputValue);
  };

  const isFormDisabled = hasSubmitted || isDisabled;

  return (
    <Card className={cn(
      "p-4 border-blue-200 bg-blue-50/50",
      isFormDisabled && "opacity-60"
    )}>
      <div className="space-y-3">
        {/* Header */}
        <div className="flex items-start gap-2">
          <UserCheck className="h-5 w-5 text-blue-600 mt-0.5" />
          <div className="flex-1">
            <h3 className="font-medium text-sm">{title}</h3>
            {toolUseId && (
              <span className="text-xs text-muted-foreground">
                ID: {toolUseId}
              </span>
            )}
          </div>
        </div>

        {/* Content based on prompt type */}
        {promptType === "buttons" && options.length > 0 && (
          <div className="space-y-2">
            {options.map((option) => (
              <Button
                key={option.value}
                onClick={() => handleButtonClick(option.value)}
                disabled={isFormDisabled}
                variant="outline"
                className="w-full justify-start text-left"
              >
                <div className="flex-1">
                  <div className="font-medium">{option.label}</div>
                  {option.description && (
                    <div className="text-xs text-muted-foreground mt-1">
                      {option.description}
                    </div>
                  )}
                </div>
              </Button>
            ))}
          </div>
        )}

        {promptType === "checkbox" && options.length > 0 && (
          <div className="space-y-3">
            <div className="space-y-2">
              {options.map((option) => (
                <div key={option.value} className="flex items-start space-x-2">
                  <input
                    type="checkbox"
                    id={option.value}
                    checked={selectedValues.includes(option.value)}
                    onChange={(e) =>
                      handleCheckboxChange(option.value, e.target.checked)
                    }
                    disabled={isFormDisabled}
                    className="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
                  />
                  <div className="flex-1">
                    <Label
                      htmlFor={option.value}
                      className={cn(
                        "text-sm font-medium cursor-pointer",
                        isFormDisabled && "cursor-not-allowed"
                      )}
                    >
                      {option.label}
                    </Label>
                    {option.description && (
                      <p className="text-xs text-muted-foreground mt-1">
                        {option.description}
                      </p>
                    )}
                  </div>
                </div>
              ))}
            </div>
            <Button
              onClick={handleCheckboxSubmit}
              disabled={isFormDisabled || selectedValues.length === 0}
              size="sm"
              className="w-full"
            >
              <Send className="h-4 w-4 mr-2" />
              Submit Selection
            </Button>
          </div>
        )}

        {promptType === "input" && (
          <div className="space-y-3">
            <Input
              type={inputType}
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              placeholder={placeholder}
              disabled={isFormDisabled}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleInputSubmit();
                }
              }}
            />
            <Button
              onClick={handleInputSubmit}
              disabled={isFormDisabled || !inputValue.trim()}
              size="sm"
              className="w-full"
            >
              <Send className="h-4 w-4 mr-2" />
              Submit
            </Button>
          </div>
        )}

        {/* Status indicator */}
        {hasSubmitted && (
          <div className="text-xs text-green-600 flex items-center gap-1">
            <UserCheck className="h-3 w-3" />
            Response submitted
          </div>
        )}
      </div>
    </Card>
  );
}