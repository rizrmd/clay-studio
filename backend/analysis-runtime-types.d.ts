/**
 * Clay Studio Analysis Runtime Type Definitions
 *
 * These types define the API available to analysis scripts running in the Clay Studio environment.
 */

/**
 * Query result from a datasource
 */
export interface QueryResult {
    rows: any[];
    columns: string[];
}

/**
 * File metadata
 */
export interface FileMetadata {
    id: string;
    name: string;
    size: number;
    type: string;
    created_at: string;
    updated_at?: string;
}

/**
 * File search result
 */
export interface FileSearchResult {
    id: string;
    name: string;
    snippet?: string;
}

/**
 * File content search match
 */
export interface FileContentMatch {
    line: number;
    content: string;
}

/**
 * Datasource metadata
 */
export interface DatasourceMetadata {
    name: string;
    type: string;
    config: Record<string, any>;
}

/**
 * File operations API
 */
export interface FileAPI {
    /**
     * List files in the project or conversation
     */
    list(conversationId?: string): Promise<FileMetadata[]>;

    /**
     * Read file content by ID
     */
    read(fileId: string): Promise<string>;

    /**
     * Search for files by name or metadata
     */
    search(query: string, options?: {
        limit?: number;
        conversationId?: string;
    }): Promise<FileSearchResult[]>;

    /**
     * Get file metadata
     */
    getMetadata(fileId: string): Promise<FileMetadata | null>;

    /**
     * Peek at the first N lines of a file
     */
    peek?(fileId: string, options?: {
        lines?: number;
        bytes?: number;
    }): Promise<string>;

    /**
     * Read a range of lines from a file
     */
    range?(fileId: string, start: number, end: number): Promise<string>;

    /**
     * Search within file content
     */
    searchContent?(fileId: string, pattern: string, options?: {
        regex?: boolean;
        caseSensitive?: boolean;
        limit?: number;
    }): Promise<FileContentMatch[]>;

    /**
     * Get download URL for a file
     */
    getDownloadUrl?(fileId: string): Promise<string>;
}

/**
 * Datasource inspection result
 */
export interface DatasourceInspection {
    tables?: Array<{
        name: string;
        columns?: Array<{
            name: string;
            type: string;
        }>;
    }>;
    schemas?: string[];
}

/**
 * Datasource operations API
 */
export interface DatasourceAPI {
    /**
     * List all available datasources in the project
     */
    list(): Promise<DatasourceMetadata[]>;

    /**
     * Get detailed information about a specific datasource
     */
    detail(name: string): Promise<DatasourceMetadata>;

    /**
     * Inspect datasource schema (tables, columns, etc.)
     */
    inspect(name: string): Promise<DatasourceInspection>;

    /**
     * Query a datasource directly (uses backend connection pooling)
     *
     * @param name Datasource name
     * @param query SQL query string
     * @param params Optional query parameters for parameterized queries
     * @param limit Maximum number of rows to return (default: 10000)
     * @returns Query result with rows and columns
     *
     * @example
     * ```typescript
     * const result = await ctx.datasource.query(
     *   'production-db',
     *   'SELECT * FROM users WHERE age > ?',
     *   [18],
     *   1000
     * );
     * ```
     */
    query(name: string, query: string, params?: any[], limit?: number): Promise<QueryResult>;
}

/**
 * Analysis execution context
 *
 * This object is passed to the `run` function of your analysis script
 * and provides access to datasources, file operations, and project metadata.
 */
export interface AnalysisContext {
    /**
     * Project ID
     */
    projectId: string;

    /**
     * Current job ID
     */
    jobId: string;

    /**
     * Available datasources for this project
     */
    datasources: Record<string, DatasourceMetadata>;

    /**
     * Metadata storage (persisted across the analysis execution)
     */
    metadata: Record<string, any>;

    /**
     * Execute a SQL query using DuckDB
     *
     * @param sql SQL query string
     * @param params Optional query parameters for parameterized queries
     * @returns Query result with rows and columns
     *
     * @example
     * ```typescript
     * const result = await ctx.query(
     *   'SELECT * FROM users WHERE age > ?',
     *   [18]
     * );
     * ```
     */
    query(sql: string, params?: any[]): Promise<QueryResult>;

    /**
     * Log a message (appears in job logs)
     */
    log(...args: any[]): void;

    /**
     * Get datasource metadata by name
     */
    getDatasource(name: string): DatasourceMetadata | null;

    /**
     * Store metadata for later retrieval
     */
    setMetadata(key: string, value: any): void;

    /**
     * Retrieve stored metadata
     */
    getMetadata(key: string): any;

    /**
     * File operations API
     */
    files: FileAPI;

    /**
     * Datasource operations API
     */
    datasource: DatasourceAPI;
}

/**
 * Parameters passed to the analysis from the user
 */
export type AnalysisParameters = Record<string, any>;

/**
 * Result returned by the analysis
 *
 * This can be any JSON-serializable value. Results larger than 10MB
 * will be rejected - use DuckDB to store large datasets instead.
 */
export type AnalysisResult = any;

/**
 * Analysis definition
 *
 * Your analysis script must export a default object implementing this interface.
 */
export interface Analysis {
    /**
     * Main analysis function
     *
     * @param ctx Analysis context providing access to datasources and APIs
     * @param parameters User-provided parameters for this analysis run
     * @returns Analysis result (must be JSON-serializable)
     *
     * @example
     * ```typescript
     * export default {
     *   async run(ctx, parameters) {
     *     const { startDate, endDate } = parameters;
     *
     *     const result = await ctx.query(
     *       'SELECT * FROM sales WHERE date BETWEEN ? AND ?',
     *       [startDate, endDate]
     *     );
     *
     *     return {
     *       totalSales: result.rows.length,
     *       data: result.rows
     *     };
     *   }
     * }
     * ```
     */
    run(ctx: AnalysisContext, parameters: AnalysisParameters): Promise<AnalysisResult>;
}

/**
 * Example analysis script structure
 */
declare const analysis: Analysis;
export default analysis;
