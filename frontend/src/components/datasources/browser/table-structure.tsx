import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Key, Link, Database } from "lucide-react";
import type { TableStructure } from "@/lib/api/datasources";
import { cn } from "@/lib/utils";

interface TableStructureViewProps {
  structure: TableStructure | null;
  loading?: boolean;
}

export function TableStructureView({
  structure,
  loading,
}: TableStructureViewProps) {
  if (loading) {
    return (
      <div className="p-4">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-muted rounded w-1/3"></div>
          <div className="space-y-2">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="h-10 bg-muted rounded"></div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (!structure || !structure.columns) {
    return (
      <div className="p-4 flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-muted-foreground">
            No structure information available
          </p>
        </div>
      </div>
    );
  }

  // Ensure all arrays exist with defaults
  const columns = structure.columns || [];
  const primaryKeys = structure.primary_keys || [];
  const foreignKeys = structure.foreign_keys || [];
  const indexes = structure.indexes || [];

  const getTypeColor = (dataType: string) => {
    const type = dataType.toLowerCase();
    if (
      type.includes("int") ||
      type.includes("serial") ||
      type.includes("number")
    ) {
      return "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300";
    }
    if (
      type.includes("varchar") ||
      type.includes("text") ||
      type.includes("char")
    ) {
      return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300";
    }
    if (type.includes("bool")) {
      return "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300";
    }
    if (
      type.includes("timestamp") ||
      type.includes("date") ||
      type.includes("time")
    ) {
      return "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300";
    }
    if (type.includes("json") || type.includes("array")) {
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300";
    }
    return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-300";
  };

  return (
    <div className="overflow-auto relative w-full h-full">
      <div className="absolute inset-0 p-4 space-y-6 flex flex-1 flex-col">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Database className="h-5 w-5 text-primary" />
          <h2 className="text-xl font-semibold">{structure.table_name}</h2>
          <Badge variant="outline" className="ml-auto">
            {columns.length} columns
          </Badge>
        </div>

        <div className="flex-1">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Nullable</TableHead>
                <TableHead>Default</TableHead>
                <TableHead>Constraints</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {columns.map((column) => (
                <TableRow key={column.name}>
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-2">
                      {column.name}
                      {column.is_primary_key && (
                        <Key className="h-3 w-3 text-yellow-600" />
                      )}
                      {column.is_foreign_key && (
                        <Link className="h-3 w-3 text-blue-600" />
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant="secondary"
                      className={cn("text-xs", getTypeColor(column.data_type))}
                    >
                      {column.data_type}
                      {column.character_maximum_length &&
                        `(${column.character_maximum_length})`}
                      {column.numeric_precision &&
                        column.numeric_scale !== undefined &&
                        `(${column.numeric_precision},${column.numeric_scale})`}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={column.is_nullable ? "outline" : "secondary"}
                    >
                      {column.is_nullable ? "NULL" : "NOT NULL"}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {column.column_default || "-"}
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      {column.is_primary_key && (
                        <Badge variant="outline" className="text-xs">
                          PK
                        </Badge>
                      )}
                      {column.is_foreign_key && (
                        <Badge variant="outline" className="text-xs">
                          FK
                        </Badge>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
        {/* Primary Keys */}
        {primaryKeys.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Key className="h-4 w-4" />
                Primary Keys
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex gap-2 flex-wrap">
                {primaryKeys.map((pk) => (
                  <Badge key={pk} variant="outline">
                    {pk}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Foreign Keys */}
        {foreignKeys.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Link className="h-4 w-4" />
                Foreign Keys
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                {foreignKeys.map((fk, index) => (
                  <div key={index} className="flex items-center gap-2 text-sm">
                    <Badge variant="outline">{fk.column_name}</Badge>
                    <span className="text-muted-foreground">references</span>
                    <Badge variant="secondary">
                      {fk.referenced_table}.{fk.referenced_column}
                    </Badge>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Indexes */}
        {indexes.length > 0 && (
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Database className="h-4 w-4" />
                Indexes
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {indexes.map((index) => (
                  <div key={index.name} className="border rounded p-3">
                    <div className="flex items-center justify-between mb-2">
                      <span className="font-medium">{index.name}</span>
                      {index.is_unique && (
                        <Badge variant="secondary" className="text-xs">
                          UNIQUE
                        </Badge>
                      )}
                    </div>
                    <div className="flex gap-1 flex-wrap">
                      {index.columns.map((column) => (
                        <Badge
                          key={column}
                          variant="outline"
                          className="text-xs"
                        >
                          {column}
                        </Badge>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
