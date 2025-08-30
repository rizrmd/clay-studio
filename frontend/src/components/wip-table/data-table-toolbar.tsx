"use client"

import { Table } from "@tanstack/react-table"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { DataTableViewOptions } from "./data-table-view-options"
import { DataTableFacetedFilter } from "./data-table-faceted-filter"
import { Download, Search, X } from "lucide-react"
import { generateCsv, download } from "export-to-csv"
import { TableConfig, TableColumn } from "./demo-data"

interface DataTableToolbarProps<TData> {
  table: Table<TData>
  globalFilter: string
  setGlobalFilter: (value: string) => void
  config?: TableConfig
  data: TData[]
  columns: TableColumn[]
}

export function DataTableToolbar<TData>({
  table,
  globalFilter,
  setGlobalFilter,
  config,
  data,
  columns,
}: DataTableToolbarProps<TData>) {
  const isFiltered = table.getState().columnFilters.length > 0 || globalFilter

  const handleExport = () => {
    const options = {
      fieldSeparator: ',',
      quoteStrings: true,
      decimalSeparator: '.',
      showLabels: true,
      showTitle: true,
      title: config?.title || 'Data Export',
      useTextFile: false,
      useBom: true,
      filename: `${config?.title?.replace(/\s+/g, '_').toLowerCase() || 'table'}_${new Date().toISOString().split('T')[0]}`,
      headers: columns.map(col => col.label),
    }

    const csv = generateCsv(options)(data as any)
    download(options)(csv)
  }

  // Get unique values for filterable columns
  const getUniqueValues = (columnKey: string) => {
    const values = new Set()
    table.getCoreRowModel().flatRows.forEach((row) => {
      const value = row.getValue(columnKey)
      if (value !== null && value !== undefined) {
        values.add(value)
      }
    })
    return Array.from(values).map(value => ({
      label: String(value),
      value: String(value),
    }))
  }

  return (
    <div className="flex items-center justify-between pb-4">
      <div className="flex flex-1 items-center space-x-2">
        {config?.enable_global_search !== false && (
          <div className="relative max-w-sm">
            <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search all columns..."
              value={globalFilter ?? ''}
              onChange={(event) => setGlobalFilter(event.target.value)}
              className="h-9 w-[150px] lg:w-[250px] pl-8"
            />
          </div>
        )}
        
        {/* Add faceted filters for specific columns */}
        {config?.enable_filter !== false && (
          <>
            {columns
              .filter(col => col.filterable)
              .map(col => {
                const column = table.getColumn(col.key)
                if (!column) return null
                
                return (
                  <DataTableFacetedFilter
                    key={col.key}
                    column={column}
                    title={col.label}
                    options={getUniqueValues(col.key)}
                  />
                )
              })}
          </>
        )}
        
        {isFiltered && (
          <Button
            variant="ghost"
            onClick={() => {
              table.resetColumnFilters()
              setGlobalFilter('')
            }}
            className="h-9 px-2 lg:px-3"
          >
            Reset
            <X className="ml-2 h-4 w-4" />
          </Button>
        )}
      </div>
      
      <div className="flex items-center space-x-2">
        {config?.enable_export !== false && (
          <Button size="sm" variant="outline" onClick={handleExport} className="h-9">
            <Download className="h-4 w-4 mr-2" />
            Export
          </Button>
        )}
        {config?.enable_column_visibility !== false && (
          <DataTableViewOptions table={table} />
        )}
      </div>
    </div>
  )
}