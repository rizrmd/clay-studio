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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Checkbox } from "@/components/ui/checkbox"
import { DataTableToolbar } from "./data-table-toolbar"
import { DataTableColumnHeader } from "./data-table-column-header"
import { cn } from "@/lib/utils"
import { TableIcon } from "lucide-react"
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

export function DataTable({ columns: columnDefs, data, config }: DataTableProps) {
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
      header: ({ column, table }) => (
        <DataTableColumnHeader 
          column={column}
          table={table}
          title={col.label}
          sortable={col.sortable !== false}
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

  // Virtual scrolling setup - always enabled
  const parentRef = React.useRef<HTMLDivElement>(null)
  
  // Always use filtered rows
  const rowsToRender = table.getFilteredRowModel().rows
  
  const virtualizer = useVirtualizer({
    count: rowsToRender.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 35,
    overscan: 10,
  })

  return (
    <div className={cn("rounded-lg border bg-card p-4")}>
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <TableIcon className="h-5 w-5 text-muted-foreground" />
          <div>
            <h3 className="font-semibold">
              {config?.title || 'Data Table'}
            </h3>
            <p className="text-sm text-muted-foreground">
              {table.getFilteredRowModel().rows.length} of {data.length} row(s)
            </p>
          </div>
        </div>
      </div>

      {/* Toolbar with search, filters, column visibility, export */}
      <DataTableToolbar 
        table={table}
        globalFilter={globalFilter}
        setGlobalFilter={setGlobalFilter}
        config={config}
        data={table.getFilteredRowModel().rows.map(row => row.original)}
        columns={columnDefs}
      />

      {/* Table */}
      <div className="rounded-md border bg-background">
        <div 
          ref={parentRef}
          className="relative overflow-auto h-[600px]"
        >
          <Table>
            <TableHeader className={cn(config?.sticky_header !== false && "sticky top-0 bg-background z-10 border-b")}>
              {table.getHeaderGroups().map((headerGroup) => (
                <TableRow key={headerGroup.id}>
                  {headerGroup.headers.map((header) => (
                    <TableHead key={header.id} style={{ width: header.column.columnDef.size }}>
                      {header.isPlaceholder
                        ? null
                        : flexRender(
                            header.column.columnDef.header,
                            header.getContext()
                          )}
                    </TableHead>
                  ))}
                </TableRow>
              ))}
            </TableHeader>
            <TableBody>
              {rowsToRender.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={tableColumns.length} className="h-24 text-center">
                    No results.
                  </TableCell>
                </TableRow>
              ) : (
                <>
                  {/* Total height container */}
                  <tr>
                    <td colSpan={tableColumns.length} style={{ height: `${virtualizer.getTotalSize()}px`, padding: 0, position: 'relative' }}>
                      <table className="w-full" style={{ position: 'absolute', top: 0, left: 0, right: 0, tableLayout: 'fixed' }}>
                        <tbody>
                          {virtualizer.getVirtualItems().map((virtualItem) => {
                            const row = rowsToRender[virtualItem.index]
                            if (!row) return null
                            return (
                              <TableRow
                                key={row.id}
                                data-state={row.getIsSelected() && "selected"}
                                style={{
                                  position: 'absolute',
                                  top: 0,
                                  left: 0,
                                  width: '100%',
                                  transform: `translateY(${virtualItem.start}px)`,
                                  height: `${virtualItem.size}px`,
                                }}
                              >
                                {row.getVisibleCells().map((cell, cellIndex) => (
                                  <TableCell 
                                    key={cell.id}
                                    style={{ 
                                      width: tableColumns[cellIndex]?.size || 'auto',
                                    }}
                                  >
                                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                                  </TableCell>
                                ))}
                              </TableRow>
                            )
                          })}
                        </tbody>
                      </table>
                    </td>
                  </tr>
                </>
              )}
            </TableBody>
          </Table>
        </div>
      </div>
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