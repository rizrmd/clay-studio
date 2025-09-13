import { ReactNode } from "react";
import { ChevronDown, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

interface AccordionSectionProps {
  title: string;
  isOpen: boolean;
  onToggle: () => void;
  children: ReactNode;
  count?: number;
  onAdd?: () => void;
  addButtonTitle?: string;
  isCollapsed?: boolean;
}

export function AccordionSection({
  title,
  isOpen,
  onToggle,
  children,
  count,
  onAdd,
  addButtonTitle = "Add",
  isCollapsed = false,
}: AccordionSectionProps) {
  if (isCollapsed) {
    return null;
  }

  return (
    <Collapsible open={isOpen} onOpenChange={onToggle}>
      <div className="border-b">
        <div className="flex items-center justify-between py-2 px-3">
          <CollapsibleTrigger asChild>
            <button className="flex items-center gap-2 text-sm font-medium hover:bg-accent/50 rounded px-1 py-1 -mx-1 transition-colors">
              <ChevronDown 
                className={cn(
                  "h-4 w-4 text-muted-foreground transition-transform duration-200",
                  !isOpen && "-rotate-90"
                )}
              />
              <span>{title}</span>
              {count !== undefined && (
                <Badge variant="secondary" className="text-xs px-1.5 py-0">
                  {count}
                </Badge>
              )}
            </button>
          </CollapsibleTrigger>
          
          {onAdd && (
            <Button
              size="sm"
              variant="ghost"
              className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={onAdd}
              title={addButtonTitle}
            >
              <Plus className="h-3 w-3" />
            </Button>
          )}
        </div>
        
        <CollapsibleContent className="data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down overflow-hidden">
          <div className="pb-2">
            {children}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
}