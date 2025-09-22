import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  dataBrowserActions,
  dataBrowserStore,
} from "@/lib/store/data-browser-store";
import { Plus, X, Square, CheckSquare } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useSnapshot } from "valtio";

interface NewRowModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function NewRowModal({ isOpen, onClose }: NewRowModalProps) {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const [formData, setFormData] = useState<Record<string, any>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Initialize form data based on table structure
  useEffect(() => {
    if (isOpen && dataBrowserSnapshot.tableStructure) {
      const initialData: Record<string, any> = {};
      dataBrowserSnapshot.tableStructure.columns.forEach((column: any) => {
        // Skip UUID columns that are likely auto-generated (like id)
        if (
          column.data_type?.toLowerCase() === "uuid" &&
          (column.name === "id" || column.column_default !== null)
        ) {
          return;
        }

        // Set default values based on data type
        switch (column.data_type?.toLowerCase()) {
          case "integer":
          case "bigint":
          case "smallint":
          case "decimal":
          case "numeric":
          case "real":
          case "double precision":
            initialData[column.name] = "";
            break;
          case "boolean":
            initialData[column.name] = false;
            break;
          case "date":
          case "timestamp":
          case "timestamptz":
            initialData[column.name] = "";
            break;
          case "uuid":
            initialData[column.name] = "";
            break;
          default:
            initialData[column.name] = "";
        }
      });
      setFormData(initialData);
      setErrors({});
    }
  }, [isOpen, dataBrowserSnapshot.tableStructure]);

  const handleInputChange = (columnName: string, value: any) => {
    setFormData((prev) => ({
      ...prev,
      [columnName]: value,
    }));

    // Clear error for this field
    if (errors[columnName]) {
      setErrors((prev) => {
        const newErrors = { ...prev };
        delete newErrors[columnName];
        return newErrors;
      });
    }
  };

  const validateForm = () => {
    const newErrors: Record<string, string> = {};

    if (!dataBrowserSnapshot.tableStructure) return false;

    dataBrowserSnapshot.tableStructure.columns.forEach((column: any) => {
      // Skip UUID columns that are auto-generated
      if (
        column.data_type?.toLowerCase() === "uuid" &&
        (column.name === "id" || column.column_default !== null)
      ) {
        return;
      }

      const value = formData[column.name];

      // Check for required fields (you might want to enhance this based on schema)
      if (
        column.nullable === false &&
        (value === "" || value === null || value === undefined)
      ) {
        newErrors[column.name] = "This field is required";
        return;
      }

      // Type validation
      if (value !== "" && value !== null && value !== undefined) {
        switch (column.data_type?.toLowerCase()) {
          case "integer":
          case "bigint":
          case "smallint":
            if (isNaN(parseInt(value))) {
              newErrors[column.name] = "Must be a valid integer";
            }
            break;
          case "decimal":
          case "numeric":
          case "real":
          case "double precision":
            if (isNaN(parseFloat(value))) {
              newErrors[column.name] = "Must be a valid number";
            }
            break;
          case "date":
          case "timestamp":
          case "timestamptz":
            if (value && isNaN(Date.parse(value))) {
              newErrors[column.name] = "Must be a valid date";
            }
            break;
        }
      }
    });

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSave = () => {
    if (!validateForm()) return;

    // Process form data to match expected types
    const processedData: Record<string, any> = {};

    if (dataBrowserSnapshot.tableStructure) {
      dataBrowserSnapshot.tableStructure.columns.forEach((column: any) => {
        // Skip UUID columns that are auto-generated
        if (
          column.data_type?.toLowerCase() === "uuid" &&
          (column.name === "id" || column.column_default !== null)
        ) {
          return;
        }

        const value = formData[column.name];

        // Skip undefined/empty values, but include explicit null values
        if (value === "" || value === undefined) {
          return;
        }

        // Include explicit null values for nullable columns
        if (value === null) {
          processedData[column.name] = null;
          return;
        }

        switch (column.data_type?.toLowerCase()) {
          case "integer":
          case "bigint":
          case "smallint":
            processedData[column.name] = parseInt(value);
            break;
          case "decimal":
          case "numeric":
          case "real":
          case "double precision":
            processedData[column.name] = parseFloat(value);
            break;
          case "boolean":
            processedData[column.name] = Boolean(value);
            break;
          default:
            processedData[column.name] = value;
        }
      });
    }

    dataBrowserActions.addNewRow(processedData);
    onClose();
  };

  const renderInput = (column: any) => {
    const value = formData[column.name];
    const error = errors[column.name];
    // const _isRequired = column.nullable === false;
    const isNullable = column.nullable !== false;

    // For nullable columns, show a checkbox to set null
    if (isNullable) {
      const isNull = value === null;
      const displayValue = isNull ? "" : value || "";

      return (
        <div className="space-y-2 relative">
          <div
            className="flex items-center absolute top-[-1.7rem] right-0 select-none"
            onClick={() => {
              if (isNull) {
                handleInputChange(column.name, "");
              } else {
                handleInputChange(column.name, null);
              }
            }}
          >
            <Badge
              variant={"outline"}
              className="flex items-center px-1 space-x-1"
            >
              {isNull ? (
                <CheckSquare className="h-4 w-4 cursor-pointer" />
              ) : (
                <Square className="h-4 w-4 cursor-pointer" />
              )}
              <div>Set Null</div>
            </Badge>
          </div>
          {isNull ? (
            <>
              <div className="flex items-center justify-center h-9 border-2 border-dashed border-muted-foreground/25 rounded-md bg-muted/20">
                <span className="text-sm text-muted-foreground">
                  {column.name} → NULL
                </span>
              </div>{" "}
            </>
          ) : (
            renderInputField(column, displayValue, error)
          )}
        </div>
      );
    }

    return renderInputField(column, value || "", error);
  };

  const renderInputField = (column: any, value: any, error: string) => {
    const inputProps = {
      id: column.name,
      value: value,
      onChange: (
        e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
      ) => handleInputChange(column.name, e.target.value),
      className: error ? "border-red-500" : "",
    };

    switch (column.data_type?.toLowerCase()) {
      case "integer":
      case "bigint":
      case "smallint":
        return (
          <Input
            {...inputProps}
            type="number"
            step="1"
            placeholder="Enter integer..."
          />
        );
      case "decimal":
      case "numeric":
      case "real":
      case "double precision":
        return (
          <Input
            {...inputProps}
            type="number"
            step="any"
            placeholder="Enter number..."
          />
        );
      case "boolean":
        return (
          <select
            id={column.name}
            value={String(value)}
            onChange={(e) =>
              handleInputChange(column.name, e.target.value === "true")
            }
            className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <option value="false">False</option>
            <option value="true">True</option>
          </select>
        );
      case "date":
        return (
          <Input {...inputProps} type="date" placeholder="Select date..." />
        );
      case "timestamp":
      case "timestamptz":
        return (
          <Input
            {...inputProps}
            type="datetime-local"
            placeholder="Select date and time..."
          />
        );
      case "text":
        return (
          <Textarea {...inputProps} placeholder="Enter text..." rows={3} />
        );
      default:
        return (
          <Input {...inputProps} type="text" placeholder="Enter value..." />
        );
    }
  };

  if (!dataBrowserSnapshot.tableStructure) {
    return null;
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="min-w-[80vw] h-[80vh] max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Plus className="h-5 w-5" />
            Add New Row
          </DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-2 py-2">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-6">
            {dataBrowserSnapshot.tableStructure.columns
              .filter(
                (column: any) =>
                  !(
                    column.data_type?.toLowerCase() === "uuid" &&
                    (column.name === "id" || column.column_default !== null)
                  )
              )
              .map((column: any) => (
                <div key={column.name} className="space-y-2 min-h-fit">
                  <Label
                    htmlFor={column.name}
                    className="text-sm font-medium flex items-center gap-1"
                  >
                    {column.name}
                    {column.nullable === false && (
                      <span
                        className="text-red-500 text-xs"
                        title="Required field"
                      >
                        *
                      </span>
                    )}
                    <span className="text-xs text-muted-foreground">
                      ({column.data_type})
                    </span>
                  </Label>
                  {renderInput(column)}
                  {errors[column.name] && (
                    <p className="text-sm text-red-500">
                      {errors[column.name]}
                    </p>
                  )}
                </div>
              ))}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            <X className="h-4 w-4 mr-1" />
            Cancel
          </Button>
          <Button onClick={handleSave}>
            <Plus className="h-4 w-4 mr-1" />
            Add Row
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
