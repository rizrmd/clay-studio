import {
  CheckSquare,
  Circle,
  Clock,
  ListTodo,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface Todo {
  content: string;
  status: "pending" | "in_progress" | "completed";
}

interface TodoListProps {
  todos: Todo[];
  className?: string;
}

export function TodoList({ todos, className }: TodoListProps) {

  return (
    <div className={cn("rounded-lg border bg-muted/30 p-3 -mt-3", className)}>
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-1.5 text-gray-600">
          <ListTodo className="h-4 w-4" />
          <span className="font-medium text-xs">Tasks</span>
        </div>
        {/* <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{completedCount}/{todos.length}</span>
          <div className="w-16 h-1.5 bg-muted rounded-full overflow-hidden">
            <div 
              className="h-full bg-purple-600 transition-all duration-300"
              style={{ width: `${completionPercentage}%` }}
            />
          </div>
        </div> */}
      </div>

      {/* Todo Items */}
      <div className="space-y-0.5 pl-3">
        {todos.map((todo, index) => (
          <div
            key={index}
            className={cn("flex items-center gap-1.5 py-0.5 text-sm")}
          >
            {todo.status === "completed" ? (
              <CheckSquare className="h-4 w-4 text-green-600 flex-shrink-0" />
            ) : todo.status === "in_progress" ? (
              <Clock className="h-3 w-3 text-amber-600 flex-shrink-0" />
            ) : (
              <Circle className="h-3 w-3 text-muted-foreground flex-shrink-0" />
            )}
            <span
              className={cn(
                todo.status === "completed" && "line-through text-green-600"
              )}
            >
              {todo.content}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
