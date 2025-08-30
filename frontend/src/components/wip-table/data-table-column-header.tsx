"use client"

import { Column, Table } from "@tanstack/react-table"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { ArrowDown, ArrowUp, ArrowUpDown, EyeOff, Filter, Check, Search } from "lucide-react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Input } from "@/components/ui/input"
import { useState, useMemo } from "react"

interface DataTableColumnHeaderProps<TData, TValue>
  extends React.HTMLAttributes<HTMLDivElement> {
  column: Column<TData, TValue>
  title: string
  sortable?: boolean
  filterable?: boolean
  table: Table<TData>
}

export function DataTableColumnHeader<TData, TValue>({
  column,
  title,
  sortable = true,
  filterable = false,
  table,
  className,
}: DataTableColumnHeaderProps<TData, TValue>) {
  const isFiltered = column.getFilterValue() !== undefined
  const [filterOpen, setFilterOpen] = useState(false)
  const [searchTerm, setSearchTerm] = useState("")

  // Get unique values for this column
  const uniqueValues = useMemo(() => {
    if (!filterable) return []
    const values = new Set<string>()
    table.getCoreRowModel().flatRows.forEach((row) => {
      const value = row.getValue(column.id)
      if (value !== null && value !== undefined) {
        values.add(String(value))
      }
    })
    return Array.from(values).sort()
  }, [filterable, table, column.id])

  // Filter values based on search term
  const filteredValues = useMemo(() => {
    if (!searchTerm) return uniqueValues
    return uniqueValues.filter(value => 
      value.toLowerCase().includes(searchTerm.toLowerCase())
    )
  }, [uniqueValues, searchTerm])

  if (!sortable && !filterable && !column.getCanHide()) {
    return <div className={cn("font-medium text-xs px-2", className)}>{title}</div>
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className={cn(
            "w-full flex items-center justify-between px-2 py-1 text-left hover:bg-muted/50 data-[state=open]:bg-muted",
            className
          )}
        >
          <span className="font-medium text-xs">{title}</span>
          <div className="flex items-center space-x-1 w-5 justify-end">
            {isFiltered && <Filter className="h-3 w-3 text-primary" />}
            {column.getIsSorted() === "desc" ? (
              <ArrowDown className="h-3 w-3" />
            ) : column.getIsSorted() === "asc" ? (
              <ArrowUp className="h-3 w-3" />
            ) : null}
          </div>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-48">
        {filterable && (
          <>
            <div className="p-2">
              <Popover open={filterOpen} onOpenChange={setFilterOpen}>
                <PopoverTrigger asChild>
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-full h-7 text-xs justify-between"
                  >
                    <span className="truncate">
                      {isFiltered ? `${column.getFilterValue()}` : "Filter values..."}
                    </span>
                    <Search className="ml-2 h-3 w-3 shrink-0" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-48 p-0" align="start">
                  <div className="p-2 border-b">
                    <Input
                      placeholder="Search values..."
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      className="h-7 text-xs"
                    />
                  </div>
                  <div className="max-h-48 overflow-auto">
                    {filteredValues.length === 0 ? (
                      <div className="p-2 text-xs text-muted-foreground">No values found</div>
                    ) : (
                      filteredValues.map((value) => (
                        <button
                          key={value}
                          onClick={() => {
                            column.setFilterValue(value === column.getFilterValue() ? undefined : value)
                            setFilterOpen(false)
                          }}
                          className="w-full flex items-center px-3 py-2 text-xs hover:bg-muted text-left"
                        >
                          <Check
                            className={cn(
                              "mr-2 h-3 w-3",
                              value === column.getFilterValue() ? "opacity-100" : "opacity-0"
                            )}
                          />
                          {value}
                        </button>
                      ))
                    )}
                  </div>
                </PopoverContent>
              </Popover>
            </div>
            <DropdownMenuSeparator />
          </>
        )}
        
        <div className="flex p-1 gap-1">
          {sortable && column.getCanSort() && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 flex-1"
              onClick={() => {
                if (column.getIsSorted() === "asc") {
                  column.toggleSorting(true)
                } else if (column.getIsSorted() === "desc") {
                  column.clearSorting()
                } else {
                  column.toggleSorting(false)
                }
              }}
            >
              {column.getIsSorted() === "desc" ? (
                <>
                  <ArrowDown className="mr-1 h-3 w-3" />
                  Z→A
                </>
              ) : column.getIsSorted() === "asc" ? (
                <>
                  <ArrowUp className="mr-1 h-3 w-3" />
                  A→Z
                </>
              ) : (
                <>
                  <ArrowUpDown className="mr-1 h-3 w-3" />
                  Sort
                </>
              )}
            </Button>
          )}
          
          {column.getCanHide() && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 flex-1"
              onClick={() => column.toggleVisibility(false)}
            >
              <EyeOff className="mr-1 h-3 w-3" />
              Hide
            </Button>
          )}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}