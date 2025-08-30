export interface TableColumn {
  key: string
  label: string
  data_type: 'string' | 'number' | 'date' | 'boolean' | 'currency'
  sortable?: boolean
  filterable?: boolean
  width?: number
  format?: string
}

export interface TableConfig {
  title?: string
  description?: string
  enable_sort?: boolean
  enable_filter?: boolean
  enable_export?: boolean
  enable_column_visibility?: boolean
  enable_row_selection?: boolean
  enable_global_search?: boolean
  sticky_header?: boolean
}

export const demoColumns: TableColumn[] = [
  {
    key: "id",
    label: "ID",
    data_type: "number",
    width: 60,
    sortable: true,
    filterable: true,
  },
  {
    key: "product",
    label: "Product",
    data_type: "string",
    filterable: true,
    sortable: true,
  },
  {
    key: "category",
    label: "Category",
    data_type: "string",
    filterable: true,
    sortable: true,
  },
  {
    key: "price",
    label: "Price",
    data_type: "currency",
    format: "currency",
    sortable: true,
  },
  {
    key: "quantity",
    label: "Quantity",
    data_type: "number",
    sortable: true,
  },
  {
    key: "revenue",
    label: "Revenue",
    data_type: "currency",
    format: "currency",
    sortable: true,
  },
  {
    key: "date",
    label: "Date",
    data_type: "date",
    sortable: true,
  },
  {
    key: "status",
    label: "Status",
    data_type: "string",
    filterable: true,
    sortable: true,
  },
  {
    key: "in_stock",
    label: "In Stock",
    data_type: "boolean",
    sortable: true,
    filterable: true,
  },
]

const products = ["Widget A", "Widget B", "Gadget X", "Gadget Y", "Tool Z", "Device M", "Component N", "Module P"]
const categories = ["Electronics", "Hardware", "Software", "Accessories", "Services", "Components", "Systems"]
const statuses = ["Pending", "Shipped", "Delivered", "Cancelled", "Returned", "Processing", "On Hold"]

export function generateDemoData(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    product: products[Math.floor(Math.random() * products.length)],
    category: categories[Math.floor(Math.random() * categories.length)],
    price: Math.random() * 1000,
    quantity: Math.floor(Math.random() * 100) + 1,
    revenue: Math.random() * 10000,
    date: new Date(2024, Math.floor(Math.random() * 12), Math.floor(Math.random() * 28) + 1).toISOString(),
    status: statuses[Math.floor(Math.random() * statuses.length)],
    in_stock: Math.random() > 0.3,
  }))
}