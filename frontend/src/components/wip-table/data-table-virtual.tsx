"use client"

import * as React from "react"
import { useState, useMemo } from "react"
import {
  ColumnDef,
  ColumnFiltersState,
  SortingState,
  VisibilityState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
  FilterFn,
} from "@tanstack/react-table"
import { useVirtualizer } from "@tanstack/react-virtual"
import { rankItem } from "@tanstack/match-sorter-utils"
import { Checkbox } from "@/components/ui/checkbox"
import { DataTableColumnHeader } from "./data-table-column-header"
import { cn } from "@/lib/utils"
import { TableColumn, TableConfig } from "./demo-data"

interface DataTableProps {
  columns: TableColumn[]
  data: any[]
  config?: TableConfig
  className?: string
}

// Fuzzy filter function
const fuzzyFilter: FilterFn<any> = (row, columnId, value, addMeta) => {
  const itemRank = rankItem(row.getValue(columnId), value)
  addMeta({ itemRank })
  return itemRank.passed
}

export function DataTable({ columns: columnDefs, data, config, className }: DataTableProps) {
  const [sorting, setSorting] = useState<SortingState>([])
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([])
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({})
  const [rowSelection, setRowSelection] = useState({})
  const [globalFilter, setGlobalFilter] = useState('')

  // Convert column definitions to TanStack format
  const columns = useMemo<ColumnDef<any>[]>(() => {
    return columnDefs.map(col => ({
      id: col.key,
      accessorKey: col.key,
      header: ({ column }) => (
        <DataTableColumnHeader 
          column={column} 
          title={col.label}
          sortable={col.sortable !== false}
          filterable={col.filterable}
          table={table}
        />
      ),
      cell: ({ row }) => {
        const value = row.getValue(col.key)
        return formatCellValue(value, col.data_type, col.format)
      },
      filterFn: col.data_type === 'number' ? 'inNumberRange' : fuzzyFilter,
      enableSorting: col.sortable !== false,
      enableHiding: config?.enable_column_visibility !== false,
      size: col.width,
    }))
  }, [columnDefs, config])

  // Add row selection column if enabled
  const tableColumns = useMemo(() => {
    if (config?.enable_row_selection) {
      return [
        {
          id: 'select',
          size: 40,
          header: ({ table }) => (
            <Checkbox
              checked={table.getIsAllPageRowsSelected()}
              onCheckedChange={(value: boolean) => table.toggleAllPageRowsSelected(!!value)}
              aria-label="Select all"
            />
          ),
          cell: ({ row }) => (
            <Checkbox
              checked={row.getIsSelected()}
              onCheckedChange={(value: boolean) => row.toggleSelected(!!value)}
              aria-label="Select row"
            />
          ),
          enableSorting: false,
          enableHiding: false,
        },
        ...columns,
      ]
    }
    return columns
  }, [columns, config?.enable_row_selection])

  const table = useReactTable({
    data,
    columns: tableColumns,
    filterFns: {
      fuzzy: fuzzyFilter,
    },
    state: {
      sorting,
      columnFilters,
      columnVisibility,
      rowSelection,
      globalFilter,
    },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    onColumnVisibilityChange: setColumnVisibility,
    onRowSelectionChange: setRowSelection,
    onGlobalFilterChange: setGlobalFilter,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    globalFilterFn: fuzzyFilter,
  })

  // Virtual scrolling setup
  const tableContainerRef = React.useRef<HTMLDivElement>(null)
  // Use sorted rows instead of just filtered rows
  const rowsToRender = table.getRowModel().rows
  
  const rowVirtualizer = useVirtualizer({
    count: rowsToRender.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 32,
    overscan: 10,
  })

  const virtualRows = rowVirtualizer.getVirtualItems()
  const totalSize = rowVirtualizer.getTotalSize()
  
  const paddingTop = virtualRows.length > 0 ? virtualRows?.[0]?.start || 0 : 0
  const paddingBottom =
    virtualRows.length > 0
      ? totalSize - (virtualRows?.[virtualRows.length - 1]?.end || 0)
      : 0

  return (
    <div 
      ref={tableContainerRef}
      className={cn("relative border bg-background overflow-auto", className)}
      style={{ height: '400px' }}
    >
        <table className="w-full">
          <thead className="sticky top-0 bg-background z-10">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="border-b">
                {headerGroup.headers.map((header) => (
                  <th 
                    key={header.id} 
                    className="h-8 px-2 text-left align-middle font-medium text-muted-foreground text-xs"
                    style={{ width: header.column.columnDef.size }}
                  >
                    {header.isPlaceholder
                      ? null
                      : flexRender(
                          header.column.columnDef.header,
                          header.getContext()
                        )}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {paddingTop > 0 && (
              <tr>
                <td style={{ height: `${paddingTop}px` }} />
              </tr>
            )}
            {virtualRows.map((virtualRow) => {
              const row = rowsToRender[virtualRow.index]
              return (
                <tr 
                  key={row.id}
                  className={cn(
                    "border-b transition-colors hover:bg-muted/50",
                    row.getIsSelected() && "bg-muted"
                  )}
                >
                  {row.getVisibleCells().map((cell) => (
                    <td 
                      key={cell.id} 
                      className="px-2 py-1 align-middle text-xs"
                      style={{ width: cell.column.columnDef.size }}
                    >
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </tr>
              )
            })}
            {paddingBottom > 0 && (
              <tr>
                <td style={{ height: `${paddingBottom}px` }} />
              </tr>
            )}
          </tbody>
        </table>
    </div>
  )
}

function formatCellValue(value: any, dataType?: string, format?: string): React.ReactNode {
  if (value === null || value === undefined) return <span className="text-muted-foreground">—</span>
  
  switch (dataType) {
    case 'date':
      return new Date(value).toLocaleDateString('en-US', { 
        year: 'numeric', 
        month: 'short', 
        day: 'numeric' 
      })
    case 'currency':
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD'
      }).format(value)
    case 'number':
      if (format === 'percentage') {
        return `${(value * 100).toFixed(2)}%`
      }
      return Number(value).toLocaleString()
    case 'boolean':
      return (
        <div className={cn(
          "inline-flex items-center justify-center rounded-full px-2 py-1 text-xs font-medium",
          value 
            ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400" 
            : "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
        )}>
          {value ? 'Yes' : 'No'}
        </div>
      )
    default:
      return String(value)
  }
}