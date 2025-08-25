import { useState } from 'react';
import { FileText } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { FileManager } from './file-manager';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';

interface FileSidebarProps {
  projectId?: string;
  conversationId?: string;
  onFileSelect?: (file: any) => void;
}

export function FileSidebar({ projectId, conversationId, onFileSelect }: FileSidebarProps) {
  const [open, setOpen] = useState(false);

  if (!projectId) return null;

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          title="View uploaded files"
        >
          <FileText className="h-4 w-4" />
        </Button>
      </SheetTrigger>
      <SheetContent side="right" className="w-[400px] sm:w-[540px]">
        <SheetHeader>
          <SheetTitle>Uploaded Files</SheetTitle>
          <SheetDescription>
            Manage files uploaded to this project. Files are automatically analyzed and can be referenced in your conversations.
          </SheetDescription>
        </SheetHeader>
        <div className="mt-6">
          <FileManager
            projectId={projectId}
            conversationId={conversationId}
            onFileSelect={(file) => {
              if (onFileSelect) {
                onFileSelect(file);
                setOpen(false);
              }
            }}
          />
        </div>
      </SheetContent>
    </Sheet>
  );
}