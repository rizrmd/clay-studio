import { useCallback } from "react";
import { ColumnFiltersState, SortingState } from "@tanstack/react-table";
import { dataBrowserStore, dataBrowserActions } from "@/lib/store/data-browser-store";
import { DataBrowserLocalStore } from "./use-local-store";

export const useServerHandlers = (localStore: DataBrowserLocalStore, datasourceId: string) => {
  // Update store state for server-side operations
  const updateStoreForServerSide = useCallback(async () => {
    if (!datasourceId || !dataBrowserStore.selectedTable) {
      return;
    }

    localStore.isServerLoading = true;

    try {
      // Update store state to match our local state
      dataBrowserStore.currentPage = localStore.currentPage;
      dataBrowserStore.pageSize = localStore.pageSize;

      // Update sorting
      if (localStore.sorting.length > 0) {
        const sort = localStore.sorting[0]; // Use first sort for now
        dataBrowserStore.sortColumn = sort.id;
        dataBrowserStore.sortDirection = sort.desc ? "desc" : "asc";
      } else {
        dataBrowserStore.sortColumn = null;
        dataBrowserStore.sortDirection = "asc";
      }

      // Update filters - create a proper filter structure
      const filters: Record<string, any> = {};

      // Add column filters directly by column name
      localStore.columnFilters.forEach((filter) => {
        filters[filter.id] = filter.value;
      });

      // Add global filter if present
      if (localStore.globalFilter) {
        filters.global = localStore.globalFilter;
      }

      console.log("Setting filters:", filters);
      console.log("Current sorting:", localStore.sorting);
      console.log("Current columnFilters:", localStore.columnFilters);
      console.log("Current globalFilter:", localStore.globalFilter);
      dataBrowserStore.filters = filters;

      // Trigger data loading with updated parameters
      await dataBrowserActions.loadTableData();
    } finally {
      localStore.isServerLoading = false;
    }
  }, [datasourceId, localStore]);

  // Server-side handlers
  const handleServerSortingChange = useCallback(
    (newSorting: SortingState) => {
      console.log("Server sorting change:", newSorting);
      localStore.sorting = newSorting;
      localStore.currentPage = 1; // Reset to first page when sorting changes
    },
    [localStore]
  );

  const handleServerFiltersChange = useCallback(
    (newFilters: ColumnFiltersState) => {
      console.log("Server filters change:", newFilters);
      localStore.columnFilters = newFilters;
      localStore.currentPage = 1; // Reset to first page when filters change
    },
    [localStore]
  );

  const handleServerGlobalFilterChange = useCallback(
    (newGlobalFilter: string) => {
      console.log("Server global filter change:", newGlobalFilter);
      localStore.globalFilter = newGlobalFilter;
      localStore.currentPage = 1; // Reset to first page when global filter changes
    },
    [localStore]
  );

  const handlePageChange = (page: number) => {
    localStore.currentPage = page;
  };

  const handlePageSizeChange = (newPageSize: number) => {
    localStore.pageSize = newPageSize;
    localStore.currentPage = 1; // Reset to first page when page size changes
  };

  return {
    updateStoreForServerSide,
    handleServerSortingChange,
    handleServerFiltersChange,
    handleServerGlobalFilterChange,
    handlePageChange,
    handlePageSizeChange,
  };
};