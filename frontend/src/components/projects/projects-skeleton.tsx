import { Skeleton } from "@/components/ui/skeleton";
import { FolderOpen, ChevronRight, Calendar, MessageSquare, Database } from "lucide-react";

export function ProjectsSkeleton() {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-6">
      {[1, 2, 3].map((index) => (
        <div
          key={index}
          className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700"
        >
          <div className="p-6">
            <div className="flex items-start justify-between mb-4">
              <div className="flex items-center">
                <FolderOpen className="h-8 w-8 text-gray-200 dark:text-gray-700" />
              </div>
              <ChevronRight className="h-5 w-5 text-gray-200 dark:text-gray-700" />
            </div>

            <Skeleton className="h-6 w-3/4 mb-2" />

            <div className="flex items-center mb-3">
              <Calendar className="h-4 w-4 mr-1 text-gray-200 dark:text-gray-700" />
              <Skeleton className="h-4 w-24" />
            </div>

            <div className="flex items-center gap-4">
              <div className="flex items-center gap-1">
                <MessageSquare className="h-4 w-4 text-gray-200 dark:text-gray-700" />
                <Skeleton className="h-4 w-12" />
              </div>
              <div className="flex items-center gap-1">
                <Database className="h-4 w-4 text-gray-200 dark:text-gray-700" />
                <Skeleton className="h-4 w-20" />
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}