import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card } from "@/components/ui/card";
import { Send, UserCheck, Check } from "lucide-react";
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
  hasResponse?: boolean;
  selectedResponse?: string | string[];
  onScroll?: () => void;
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
  hasResponse = false,
  selectedResponse,
  onScroll,
}: AskUserProps) {
  const [selectedValues, setSelectedValues] = useState<string[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [hasSubmitted, setHasSubmitted] = useState(hasResponse);

  // Determine the selected option from the response
  const [selectedOption, setSelectedOption] = useState<string | null>(() => {
    console.log("AskUser init:", {
      hasResponse,
      selectedResponse,
      promptType,
      options,
    });
    if (
      hasResponse &&
      typeof selectedResponse === "string" &&
      promptType === "buttons"
    ) {
      // Find the option that matches the response content
      const matchingOption = options.find(
        (opt) =>
          opt.label === selectedResponse || opt.value === selectedResponse
      );
      console.log("AskUser matching option:", {
        selectedResponse,
        matchingOption,
      });
      return matchingOption ? matchingOption.value : null;
    }
    return null;
  });

  const handleCheckboxChange = (value: string, checked: boolean) => {
    if (checked) {
      setSelectedValues([...selectedValues, value]);
    } else {
      setSelectedValues(selectedValues.filter((v) => v !== value));
    }
  };

  const handleButtonClick = (value: string) => {
    if (hasSubmitted || isDisabled) return;
    setSelectedOption(value);
    setHasSubmitted(true);
    onSubmit(value);

    // Scroll to bottom after a short delay to show the response
    if (onScroll) {
      setTimeout(() => {
        onScroll();
      }, 1000);
    }
  };

  const handleCheckboxSubmit = () => {
    if (hasSubmitted || isDisabled || selectedValues.length === 0) return;
    setHasSubmitted(true);
    onSubmit(selectedValues);

    // Scroll to bottom after a short delay to show the response
    if (onScroll) {
      setTimeout(() => {
        onScroll();
      }, 1000);
    }
  };

  const handleInputSubmit = () => {
    if (hasSubmitted || isDisabled || !inputValue.trim()) return;
    setHasSubmitted(true);
    onSubmit(inputValue);

    // Scroll to bottom after a short delay to show the response
    if (onScroll) {
      setTimeout(() => {
        onScroll();
      }, 1000);
    }
  };

  const isFormDisabled = hasSubmitted || isDisabled || hasResponse;

  return (
    <Card className={cn("p-4 border-blue-200 bg-blue-50/50")}>
      <div className="space-y-3">
        {/* Header */}
        <div className="flex items-start gap-2">
          <UserCheck className="h-5 w-5 text-blue-600 mt-0.5" />
          <div className="flex-1">
            <h3 className="font-medium text-sm">{title}</h3>
            {/* {toolUseId && (
              <span className="text-xs text-muted-foreground">
                ID: {toolUseId}
              </span>
            )} */}
          </div>
        </div>

        {/* Content based on prompt type */}
        {promptType === "buttons" && options.length > 0 && (
          <div className="space-y-2">
            {options.map((option) => (
              <Button
                key={option.value}
                onClick={() => {
                  if (isFormDisabled) return;
                  handleButtonClick(option.value);
                }}
                variant="outline"
                className={cn(
                  "w-full justify-start text-left",
                  option.description && "min-h-[60px]",
                  selectedOption === option.value &&
                    "bg-green-50 border-green-200 hover:bg-green-100",
                  isFormDisabled &&
                    selectedOption !== option.value &&
                    "opacity-40",
                  isFormDisabled && "cursor-default"
                )}
              >
                <div className="flex w-full items-center">
                  <div className="flex-1">
                    <div className="font-medium">{option.label}</div>
                    {option.description && (
                      <div className="text-xs text-muted-foreground mt-1">
                        {option.description}
                      </div>
                    )}
                  </div>
                  {selectedOption === option.value && (
                    <Check className="h-4 w-4 text-green-600 ml-2" />
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
              className="bg-white"
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

        {/* Status indicator for input and checkbox types */}
        {hasSubmitted && promptType !== "buttons" && (
          <div className="text-xs text-green-600 flex items-center gap-1">
            <UserCheck className="h-3 w-3" />
            Response submitted
          </div>
        )}
      </div>
    </Card>
  );
}
