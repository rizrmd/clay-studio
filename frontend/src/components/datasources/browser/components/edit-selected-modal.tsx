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
import { 
  Edit, 
  X, 
  Square, 
  CheckSquare, 
  ChevronLeft, 
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Save
} from "lucide-react";
import React, { useEffect, useState, useCallback } from "react";
import { useSnapshot } from "valtio";
import { datasourcesApi } from "@/lib/api/datasources";

interface EditSelectedModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedRowIds: string[];
  selectedRows: Record<string, boolean>;
}

export function EditSelectedModal({ 
  isOpen, 
  onClose, 
  selectedRowIds
}: EditSelectedModalProps) {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const [currentRowIndex, setCurrentRowIndex] = useState(0);
  const [currentRowData, setCurrentRowData] = useState<any>(null);
  const [formData, setFormData] = useState<Record<string, any>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [modifiedRows, setModifiedRows] = useState<Record<string, any>>({});

  const currentRowId = selectedRowIds[currentRowIndex];
  const totalRows = selectedRowIds.length;

  // Fetch data for a single row
  const fetchSingleRowData = useCallback(async (rowId: string) => {
    if (!dataBrowserSnapshot.selectedDatasourceId || !dataBrowserSnapshot.selectedTable || !dataBrowserSnapshot.tableStructure) {
      return null;
    }

    try {
      // First check if the row is already in the current page data
      const dataArray = (dataBrowserSnapshot.tableData as any)?.data || dataBrowserSnapshot.tableData?.rows;
      
      if (dataArray && Array.isArray(dataArray)) {
        const matchingRow = dataArray.find((row: any) => {
          const extractedRowId = Array.isArray(row) ? row[0] : row?.id || row?.[Object.keys(row)[0]];
          return String(extractedRowId) === String(rowId);
        });
        
        if (matchingRow) {
          const columns = dataBrowserSnapshot.tableStructure.columns;
          const columnNames = columns.map(col => col.name);
          const rowObject: Record<string, any> = {};
          
          if (Array.isArray(matchingRow)) {
            columnNames.forEach((colName, index) => {
              rowObject[colName] = matchingRow[index];
            });
          } else {
            Object.assign(rowObject, matchingRow);
          }
          
          console.log(`Found row ${rowId} in current page:`, rowObject);
          return rowObject;
        }
      }
      
      // If not in current page, fetch it from the server
      console.log(`Row ${rowId} not in current page, fetching from server...`);
      
      // Use a filter query to get just this specific row
      const result = await datasourcesApi.getTableData(
        dataBrowserSnapshot.selectedDatasourceId,
        dataBrowserSnapshot.selectedTable,
        {
          page: 1,
          limit: 1,
          filters: { id: rowId } // Assuming 'id' is the primary key
        }
      );
      
      if (result.rows && result.rows.length > 0) {
        const columns = dataBrowserSnapshot.tableStructure.columns;
        const columnNames = columns.map(col => col.name);
        const rowObject: Record<string, any> = {};
        const row = result.rows[0];
        
        if (Array.isArray(row)) {
          columnNames.forEach((colName, index) => {
            rowObject[colName] = row[index];
          });
        } else {
          Object.assign(rowObject, row);
        }
        
        console.log(`Fetched row ${rowId} from server:`, rowObject);
        return rowObject;
      }
      
      console.warn(`No data found for row ID: ${rowId}`);
      return null;
    } catch (error) {
      console.error(`Failed to fetch row ${rowId}:`, error);
      return null;
    }
  }, [dataBrowserSnapshot.selectedDatasourceId, dataBrowserSnapshot.selectedTable, dataBrowserSnapshot.tableStructure, dataBrowserSnapshot.tableData]);

  // Load row data when modal opens or row changes
  useEffect(() => {
    if (!isOpen || !currentRowId) {
      return;
    }
    
    const loadRowData = async () => {
      // Check if we have unsaved changes for the previous row
      if (modifiedRows[currentRowId]) {
        // Load from modified data
        setFormData({ ...modifiedRows[currentRowId] });
        setCurrentRowData(modifiedRows[currentRowId]);
        return;
      }
      
      setIsLoading(true);
      setErrors({});
      
      try {
        const data = await fetchSingleRowData(currentRowId);
        if (data) {
          setCurrentRowData(data);
          setFormData({ ...data });
        } else {
          console.error(`Could not load data for row ${currentRowId}`);
        }
      } catch (error) {
        console.error('Error loading row data:', error);
      } finally {
        setIsLoading(false);
      }
    };
    
    loadRowData();
  }, [isOpen, currentRowId, fetchSingleRowData, modifiedRows]);

  // Reset when modal opens
  useEffect(() => {
    if (isOpen && selectedRowIds.length > 0) {
      setCurrentRowIndex(0);
      setModifiedRows({});
      setErrors({});
    }
  }, [isOpen, selectedRowIds]);

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
      const value = formData[column.name];

      // Check for required fields
      if (
        column.is_nullable === false &&
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

  const handleSaveCurrentRow = async () => {
    if (!validateForm() || !currentRowId) return;

    setIsSaving(true);
    try {
      // Process form data to match expected types
      const processedData: Record<string, any> = {};

      if (dataBrowserSnapshot.tableStructure) {
        dataBrowserSnapshot.tableStructure.columns.forEach((column: any) => {
          const value = formData[column.name];

          // Include all values, even empty ones, to allow clearing fields
          if (value === null) {
            processedData[column.name] = null;
            return;
          }

          switch (column.data_type?.toLowerCase()) {
            case "integer":
            case "bigint":
            case "smallint":
              processedData[column.name] = value === "" ? null : parseInt(value);
              break;
            case "decimal":
            case "numeric":
            case "real":
            case "double precision":
              processedData[column.name] = value === "" ? null : parseFloat(value);
              break;
            case "boolean":
              processedData[column.name] = Boolean(value);
              break;
            default:
              processedData[column.name] = value === "" ? null : value;
          }
        });
      }

      // Use the bulk update API with a single row
      const primaryKeyColumn = dataBrowserSnapshot.tableStructure?.primary_keys?.[0] || "id";
      
      await datasourcesApi.updateRows(
        dataBrowserSnapshot.selectedDatasourceId!,
        dataBrowserSnapshot.selectedTable!,
        {
          updates: { [currentRowId]: processedData },
          id_column: primaryKeyColumn
        }
      );

      // Clear from modified rows since it's saved
      setModifiedRows(prev => {
        const updated = { ...prev };
        delete updated[currentRowId];
        return updated;
      });

      console.log(`Successfully updated row ${currentRowId}`);
      return true;
    } catch (error) {
      console.error('Failed to update row:', error);
      // TODO: Show proper error message to user
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveAndNext = async () => {
    const saved = await handleSaveCurrentRow();
    if (saved && currentRowIndex < totalRows - 1) {
      setCurrentRowIndex(prev => prev + 1);
    }
  };

  const handleSaveAndPrev = async () => {
    const saved = await handleSaveCurrentRow();
    if (saved && currentRowIndex > 0) {
      setCurrentRowIndex(prev => prev - 1);
    }
  };

  const handleSaveAndClose = async () => {
    await handleSaveCurrentRow();
    onClose();
    // Trigger data refresh
    dataBrowserActions.loadTableData();
  };

  const saveCurrentChanges = () => {
    // Save current form data to modifiedRows for later
    if (currentRowId && JSON.stringify(formData) !== JSON.stringify(currentRowData)) {
      setModifiedRows(prev => ({
        ...prev,
        [currentRowId]: { ...formData }
      }));
    }
  };

  const handleNext = () => {
    if (currentRowIndex < totalRows - 1) {
      saveCurrentChanges();
      setCurrentRowIndex(prev => prev + 1);
    }
  };

  const handlePrev = () => {
    if (currentRowIndex > 0) {
      saveCurrentChanges();
      setCurrentRowIndex(prev => prev - 1);
    }
  };

  const handleFirst = () => {
    if (currentRowIndex !== 0) {
      saveCurrentChanges();
      setCurrentRowIndex(0);
    }
  };

  const handleLast = () => {
    if (currentRowIndex !== totalRows - 1) {
      saveCurrentChanges();
      setCurrentRowIndex(totalRows - 1);
    }
  };

  const renderInput = (column: any) => {
    const value = formData[column.name];
    const error = errors[column.name];
    const isNullable = column.is_nullable !== false;

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
              </div>
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

  if (!dataBrowserSnapshot.tableStructure || selectedRowIds.length === 0) {
    return null;
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="min-w-[80vw] h-[80vh] max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Edit className="h-5 w-5" />
            Edit Selected Rows
            <Badge variant="secondary" className="ml-2">
              {currentRowIndex + 1} of {totalRows}
            </Badge>
            {Object.keys(modifiedRows).length > 0 && (
              <Badge variant="destructive" className="ml-2">
                {Object.keys(modifiedRows).length} unsaved
              </Badge>
            )}
          </DialogTitle>
          {totalRows > 1 && (
            <div className="flex items-center gap-2 mt-2">
              <Button
                variant="outline"
                size="sm"
                onClick={handleFirst}
                disabled={currentRowIndex === 0 || isLoading}
                title="Jump to first row"
              >
                <ChevronsLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handlePrev}
                disabled={currentRowIndex === 0 || isLoading}
              >
                <ChevronLeft className="h-4 w-4 mr-1" />
                Previous
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleNext}
                disabled={currentRowIndex === totalRows - 1 || isLoading}
              >
                Next
                <ChevronRight className="h-4 w-4 ml-1" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleLast}
                disabled={currentRowIndex === totalRows - 1 || isLoading}
                title="Jump to last row"
              >
                <ChevronsRight className="h-4 w-4" />
              </Button>
              <div className="text-sm text-muted-foreground ml-4">
                Row ID: {currentRowId}
              </div>
            </div>
          )}
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-2 py-2">
          {isLoading ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-center">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-2"></div>
                <p className="text-muted-foreground">Loading row data...</p>
              </div>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-6">
              {dataBrowserSnapshot.tableStructure.columns.map((column: any) => (
                <div key={column.name} className="space-y-2 min-h-fit">
                  <Label
                    htmlFor={column.name}
                    className="text-sm font-medium flex items-center gap-1"
                  >
                    {column.name}
                    {column.is_nullable === false && (
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
          )}
        </div>

        <DialogFooter className="flex justify-between">
          <div className="flex gap-2">
            {totalRows > 1 && (
              <>
                <Button 
                  variant="outline" 
                  onClick={handleSaveAndPrev}
                  disabled={currentRowIndex === 0 || isSaving || isLoading}
                >
                  <Save className="h-4 w-4 mr-1" />
                  Save & Previous
                </Button>
                <Button 
                  variant="outline" 
                  onClick={handleSaveAndNext}
                  disabled={currentRowIndex === totalRows - 1 || isSaving || isLoading}
                >
                  <Save className="h-4 w-4 mr-1" />
                  Save & Next
                </Button>
              </>
            )}
            {Object.keys(modifiedRows).length > 0 && (
              <Button
                variant="default"
                onClick={async () => {
                  setIsSaving(true);
                  try {
                    // Save all modified rows at once
                    const primaryKeyColumn = dataBrowserSnapshot.tableStructure?.primary_keys?.[0] || "id";
                    await datasourcesApi.updateRows(
                      dataBrowserSnapshot.selectedDatasourceId!,
                      dataBrowserSnapshot.selectedTable!,
                      {
                        updates: modifiedRows,
                        id_column: primaryKeyColumn
                      }
                    );
                    setModifiedRows({});
                    console.log(`Successfully saved ${Object.keys(modifiedRows).length} rows`);
                  } catch (error) {
                    console.error('Failed to save all rows:', error);
                  } finally {
                    setIsSaving(false);
                  }
                }}
                disabled={isSaving}
              >
                <Save className="h-4 w-4 mr-1" />
                Save All ({Object.keys(modifiedRows).length})
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={onClose}>
              <X className="h-4 w-4 mr-1" />
              Cancel
            </Button>
            <Button 
              onClick={handleSaveAndClose}
              disabled={isSaving || isLoading}
            >
              <Save className="h-4 w-4 mr-1" />
              {isSaving ? "Saving..." : "Save Current & Close"}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}