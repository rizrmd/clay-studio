import { Skeleton } from "@/components/ui/skeleton";
import { Bot, User } from "lucide-react";

export function ChatSkeleton() {
  return (
    <div className="flex-1 overflow-hidden relative">
      <div className="h-full overflow-y-auto pb-24 sm:pb-20">
        <div className="px-4 py-4 space-y-4 max-w-[44rem] mx-auto">
          {/* User message skeleton */}
          <div className="flex gap-3 justify-end">
            <div className="flex-1 max-w-[85%]">
              <div className="flex flex-col items-end">
                <div className="rounded-lg px-3 py-2 w-full">
                  <Skeleton className="h-4 w-3/4 mb-2" />
                  <Skeleton className="h-4 w-full" />
                </div>
                <Skeleton className="h-3 w-16 mt-1" />
              </div>
            </div>
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md">
              <User className="h-4 w-4 text-primary/30" />
            </div>
          </div>

          {/* Assistant message skeleton */}
          <div className="flex gap-3">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md">
              <Bot className="h-4 w-4 text-secondary/30" />
            </div>
            <div className="flex-1 max-w-[85%]">
              <div className="rounded-lg px-3 py-2 space-y-2">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-5/6" />
                <Skeleton className="h-4 w-4/6" />
              </div>
              <Skeleton className="h-3 w-16 mt-1" />
            </div>
          </div>

          {/* Another user message skeleton */}
          <div className="flex gap-3 justify-end">
            <div className="flex-1 max-w-[85%]">
              <div className="flex flex-col items-end">
                <div className="rounded-lg px-3 py-2 w-full">
                  <Skeleton className="h-4 w-2/3" />
                </div>
                <Skeleton className="h-3 w-16 mt-1" />
              </div>
            </div>
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md">
              <User className="h-4 w-4 text-primary/30" />
            </div>
          </div>

          {/* Assistant message skeleton with longer content */}
          <div className="flex gap-3">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md">
              <Bot className="h-4 w-4 text-secondary/30" />
            </div>
            <div className="flex-1 max-w-[85%]">
              <div className="rounded-lg px-3 py-2 space-y-2">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <div className="mt-3">
                  <Skeleton className="h-20 w-full rounded" />
                </div>
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-2/3" />
              </div>
              <div className="flex items-center gap-2 mt-1">
                <Skeleton className="h-5 w-24 rounded-sm" />
                <Skeleton className="h-3 w-16" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}