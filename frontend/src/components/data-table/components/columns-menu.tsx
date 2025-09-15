"use client";

import { Table } from "@tanstack/react-table";
import { MoreVertical, Eye } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";

interface DataTableColumnsMenuProps<TData> {
  table: Table<TData>;
}

export function DataTableColumnsMenu<TData>({
  table,
}: DataTableColumnsMenuProps<TData>) {
  const hiddenColumns = table
    .getAllColumns()
    .filter(
      (column) =>
        !column.getIsVisible() &&
        column.getCanHide() &&
        column.id !== "select" // Don't show the select column
    );

  if (hiddenColumns.length === 0) {
    return null;
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0 ml-2"
          title="Show hidden columns"
        >
          <MoreVertical className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel className="text-xs font-normal text-muted-foreground">
          Hidden Columns
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {hiddenColumns.map((column) => {
          const columnDef = column.columnDef as any;
          const title = columnDef.meta?.title || column.id;
          
          return (
            <DropdownMenuItem
              key={column.id}
              onClick={() => column.toggleVisibility(true)}
              className="cursor-pointer"
            >
              <Eye className="mr-2 h-4 w-4" />
              <span>Show {title}</span>
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}