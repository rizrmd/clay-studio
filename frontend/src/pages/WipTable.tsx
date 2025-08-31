import { DataTable } from "@/components/wip-table/data-table-virtual"
import { generateDemoData, demoColumns } from "@/components/wip-table/demo-data"
import { useState } from "react"
import { Card } from "@/components/ui/card"

export function WipTablePage() {
  const [dataSize, setDataSize] = useState(100)
  
  const data = generateDemoData(dataSize)
  
  const config = {
    title: "Sales Performance Dashboard",
    description: "Q4 2024 Sales Data",
    enable_sort: true,
    enable_filter: true,
    enable_export: true,
    enable_column_visibility: true,
    enable_row_selection: false,
    sticky_header: true,
  }

  return (
    <div className="container mx-auto py-10">
      <h1 className="text-3xl font-bold mb-6">WIP Table Component</h1>
      
      {/* Controls */}
      <Card className="p-4 mb-6">
        <h2 className="text-lg font-semibold mb-4">Demo Controls</h2>
        <div className="flex gap-4 flex-wrap items-center">
          <div className="flex items-center gap-2">
            <label>Data Size:</label>
            <select 
              value={dataSize} 
              onChange={(e) => setDataSize(Number(e.target.value))}
              className="border rounded px-2 py-1"
            >
              <option value={10}>10 rows</option>
              <option value={100}>100 rows</option>
              <option value={1000}>1,000 rows</option>
              <option value={10000}>10,000 rows</option>
              <option value={50000}>50,000 rows</option>
            </select>
          </div>
          <div className="text-sm text-muted-foreground">
            Virtual scrolling is always enabled for optimal performance
          </div>
        </div>
      </Card>

      {/* Table */}
      <DataTable 
        columns={demoColumns}
        data={data}
        config={config}
        className="h-[300px]"
      />
    </div>
  )
}