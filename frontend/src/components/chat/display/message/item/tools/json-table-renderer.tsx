import * as React from "react";
import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { ChevronDown, ChevronRight, Eye, Copy, Check } from "lucide-react";

interface JsonTableRendererProps {
  data: any;
  title?: string;
}

interface JsonValue {
  key: string;
  value: any;
  type: string;
  isLong: boolean;
  preview: string;
}

const MAX_PREVIEW_LENGTH = 100;

function getValueType(value: any): string {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  if (typeof value === "object") return "object";
  return typeof value;
}

function formatPreview(value: any): string {
  if (value === null || value === undefined) return "null";
  
  const str = typeof value === "string" ? value : JSON.stringify(value);
  
  if (str.length <= MAX_PREVIEW_LENGTH) {
    return str;
  }
  
  return str.substring(0, MAX_PREVIEW_LENGTH) + "...";
}

function isLongContent(value: any): boolean {
  const str = typeof value === "string" ? value : JSON.stringify(value);
  return str.length > MAX_PREVIEW_LENGTH;
}

function processJsonData(data: any): JsonValue[] {
  if (data === null || data === undefined) {
    return [];
  }

  if (Array.isArray(data)) {
    return data.map((item, index) => ({
      key: `[${index}]`,
      value: item,
      type: getValueType(item),
      isLong: isLongContent(item),
      preview: formatPreview(item),
    }));
  }

  if (typeof data === "object") {
    return Object.entries(data).map(([key, value]) => ({
      key,
      value,
      type: getValueType(value),
      isLong: isLongContent(value),
      preview: formatPreview(value),
    }));
  }

  return [{
    key: "value",
    value: data,
    type: getValueType(data),
    isLong: isLongContent(data),
    preview: formatPreview(data),
  }];
}

function ExpandableValueModal({ value, keyName }: { value: any; keyName: string }) {
  const [copied, setCopied] = useState(false);

  const copyToClipboard = async () => {
    try {
      const text = typeof value === "string" ? value : JSON.stringify(value, null, 2);
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  const formattedValue = typeof value === "string" ? value : JSON.stringify(value, null, 2);

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="sm" className="h-6 w-6 p-0">
          <Eye className="h-3 w-3" />
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-4xl max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between pr-8">
            <span>Field: {keyName}</span>
            <Button
              variant="outline"
              size="sm"
              onClick={copyToClipboard}
              className="h-8 px-2 mr-2"
            >
              {copied ? (
                <Check className="h-3 w-3" />
              ) : (
                <Copy className="h-3 w-3" />
              )}
            </Button>
          </DialogTitle>
        </DialogHeader>
        <div className="flex-1 overflow-auto border rounded-lg bg-muted/30">
          <pre className="p-4 text-xs font-mono whitespace-pre-wrap">
            {formattedValue}
          </pre>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function NestedJsonRenderer({ data, depth = 0 }: { data: any; depth?: number }) {
  const [isOpen, setIsOpen] = useState(false);

  if (data === null || data === undefined) {
    return <span className="text-muted-foreground">null</span>;
  }

  if (typeof data === "string" || typeof data === "number" || typeof data === "boolean") {
    return <span>{String(data)}</span>;
  }

  if (Array.isArray(data) || typeof data === "object") {
    return (
      <Button variant="ghost" className="h-auto p-1 text-xs" onClick={() => setIsOpen(!isOpen)}>
        {isOpen ? (
          <ChevronDown className="h-3 w-3" />
        ) : (
          <ChevronRight className="h-3 w-3" />
        )}
        <Badge variant="secondary" className="ml-1 text-xs">
          {Array.isArray(data) ? `Array[${data.length}]` : `Object{${Object.keys(data).length}}`}
        </Badge>
      </Button>
    );
  }

  return <span>{String(data)}</span>;
}

function TableRowWithExpansion({ item, index }: { item: JsonValue; index: number }) {
  const [isExpanded, setIsExpanded] = useState(false);

  const handleToggleExpansion = () => {
    if (item.type === "object" || item.type === "array") {
      setIsExpanded(!isExpanded);
    }
  };

  const expandedItems = item.type === "object" || item.type === "array" ? processJsonData(item.value) : [];

  return (
    <>
      <TableRow key={index}>
        <TableCell className="font-mono text-xs max-w-[200px] truncate p-1 px-2">{item.key}</TableCell>
        <TableCell className="max-w-[100px] p-1 px-2">
          <Badge variant="outline" className="text-xs">
            {item.type}
          </Badge>
        </TableCell>
        <TableCell className="font-mono text-xs max-w-[200px] p-1 px-2">
          {item.type === "object" || item.type === "array" ? (
            <Button variant="ghost" className="h-auto p-1 text-xs" onClick={handleToggleExpansion}>
              {isExpanded ? (
                <ChevronDown className="h-3 w-3" />
              ) : (
                <ChevronRight className="h-3 w-3" />
              )}
              <Badge variant="secondary" className="ml-1 text-xs">
                {Array.isArray(item.value) ? `Array[${item.value.length}]` : `Object{${Object.keys(item.value).length}}`}
              </Badge>
            </Button>
          ) : (
            <span className="truncate block">{item.preview}</span>
          )}
        </TableCell>
        <TableCell className="max-w-[50px] p-1 px-2">
          {item.isLong && (
            <ExpandableValueModal value={item.value} keyName={item.key} />
          )}
        </TableCell>
      </TableRow>
      {isExpanded && (
        <TableRow>
          <TableCell colSpan={4} className="p-0">
            <div className="bg-muted/30 border-t">
              <div className="p-2">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-[200px] text-xs p-1 px-2">Key</TableHead>
                      <TableHead className="w-[100px] text-xs p-1 px-2">Type</TableHead>
                      <TableHead className="text-xs p-1 px-2">Value</TableHead>
                      <TableHead className="w-[50px] text-xs p-1 px-2">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {expandedItems.map((nestedItem, nestedIndex) => (
                      <TableRowWithExpansion key={nestedIndex} item={nestedItem} index={nestedIndex} />
                    ))}
                  </TableBody>
                </Table>
              </div>
            </div>
          </TableCell>
        </TableRow>
      )}
    </>
  );
}

export function JsonTableRenderer({ data, title }: JsonTableRendererProps) {
  if (!data) {
    return (
      <div className="text-muted-foreground text-sm">
        No data to display
      </div>
    );
  }

  const items = processJsonData(data);

  if (items.length === 0) {
    return (
      <div className="text-muted-foreground text-sm">
        Empty data
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {title && <h4 className="font-medium text-sm">{title}</h4>}
      <div className="border rounded-lg overflow-auto max-h-[500px] max-w-full">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[200px] max-w-[200px] text-xs sticky top-0 bg-background p-1 px-2">Key</TableHead>
              <TableHead className="w-[100px] max-w-[100px] text-xs sticky top-0 bg-background p-1 px-2">Type</TableHead>
              <TableHead className="text-xs sticky top-0 bg-background max-w-[200px] p-1 px-2">Value</TableHead>
              <TableHead className="w-[50px] max-w-[50px] text-xs sticky top-0 bg-background p-1 px-2">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((item, index) => (
              <TableRowWithExpansion key={index} item={item} index={index} />
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}