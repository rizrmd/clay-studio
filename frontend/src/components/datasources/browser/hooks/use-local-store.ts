import { useState } from "react";
import { useSnapshot, proxy } from "valtio";
import { ColumnFiltersState, SortingState } from "@tanstack/react-table";

// Local component store using valtio
export interface DataBrowserLocalStore {
  currentMode: "data" | "structure";
  currentPage: number;
  pageSize: number;
  sorting: SortingState;
  columnFilters: ColumnFiltersState;
  globalFilter: string;
  isServerLoading: boolean;
  selectedRows: Record<string, boolean>;
}

const createLocalStore = (mode: "data" | "structure"): DataBrowserLocalStore =>
  proxy({
    currentMode: mode,
    currentPage: 1,
    pageSize: 50, // Default to 50 as requested
    sorting: [],
    columnFilters: [],
    globalFilter: "",
    isServerLoading: false,
    selectedRows: {},
  });

export const useLocalStore = (mode: "data" | "structure" = "data") => {
  const [localStore] = useState(() => createLocalStore(mode));
  const localSnapshot = useSnapshot(localStore);
  
  return { localStore, localSnapshot };
};