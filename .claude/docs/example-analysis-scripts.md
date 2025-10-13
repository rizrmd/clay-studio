# Example Analysis Scripts

## Basic Analysis

```typescript
export default {
  async run(ctx, parameters) {
    ctx.log('Starting analysis...');

    // Use parameters
    const { startDate, endDate } = parameters;

    return {
      message: 'Analysis completed',
      dates: { startDate, endDate }
    };
  }
}
```

## DuckDB Query Example

```typescript
export default {
  async run(ctx, parameters) {
    // Create a temporary table
    await ctx.query(`
      CREATE TABLE sales AS
      SELECT
        1 as id,
        'Product A' as product,
        100.50 as amount,
        '2024-01-01' as date
    `);

    // Query the data
    const result = await ctx.query(`
      SELECT
        product,
        SUM(amount) as total
      FROM sales
      GROUP BY product
    `);

    return {
      sales: result.rows,
      totalRows: result.rows.length
    };
  }
}
```

## External Datasource Query

```typescript
export default {
  async run(ctx, parameters) {
    // Check available datasources
    const pgDatasource = ctx.getDatasource('production_db');
    if (!pgDatasource) {
      throw new Error('production_db datasource not configured');
    }

    ctx.log('Querying PostgreSQL database...');

    // Query external PostgreSQL database
    const users = await ctx.queryDatasource(
      'production_db',
      'SELECT id, name, email FROM users WHERE created_at > $1',
      [parameters.since]
    );

    return {
      userCount: users.rows.length,
      users: users.rows
    };
  }
}
```

## Load and Transform Data

```typescript
export default {
  async run(ctx, parameters) {
    // Load data from external source
    const salesData = await ctx.queryDatasource(
      'mysql_sales',
      'SELECT * FROM orders WHERE date BETWEEN ? AND ?',
      [parameters.startDate, parameters.endDate]
    );

    // Load into DuckDB for analysis
    await ctx.loadData('orders', salesData.rows);

    // Perform complex analytics
    const analysis = await ctx.query(`
      SELECT
        DATE_TRUNC('month', date) as month,
        COUNT(*) as order_count,
        SUM(amount) as total_revenue,
        AVG(amount) as avg_order_value
      FROM orders
      GROUP BY month
      ORDER BY month
    `);

    return {
      monthlyStats: analysis.rows
    };
  }
}
```

## File Processing

```typescript
export default {
  async run(ctx, parameters) {
    // List files in conversation
    const files = await ctx.files.list(parameters.conversationId);

    ctx.log(`Found ${files.length} files`);

    // Process CSV files
    const csvFiles = files.filter(f => f.name.endsWith('.csv'));

    const results = [];
    for (const file of csvFiles) {
      const content = await ctx.files.read(file.id);

      // Parse CSV using DuckDB
      await ctx.query(`
        CREATE TEMP TABLE file_data AS
        SELECT * FROM read_csv_auto('${file.name}')
      `);

      const stats = await ctx.query(`
        SELECT
          COUNT(*) as row_count,
          COUNT(DISTINCT *) as unique_rows
        FROM file_data
      `);

      results.push({
        fileName: file.name,
        stats: stats.rows[0]
      });

      // Clean up
      await ctx.query('DROP TABLE file_data');
    }

    return {
      filesProcessed: results
    };
  }
}
```

## Advanced: Multi-Source Join

```typescript
export default {
  async run(ctx, parameters) {
    // Get data from multiple sources
    ctx.log('Fetching data from PostgreSQL...');
    const customers = await ctx.queryDatasource(
      'crm_postgres',
      'SELECT id, name, segment FROM customers WHERE active = true'
    );

    ctx.log('Fetching data from MySQL...');
    const orders = await ctx.queryDatasource(
      'orders_mysql',
      'SELECT customer_id, order_id, amount, date FROM orders WHERE date >= ?',
      [parameters.since]
    );

    // Load both into DuckDB
    await ctx.loadData('customers', customers.rows);
    await ctx.loadData('orders', orders.rows);

    // Perform join and analysis
    const result = await ctx.query(`
      SELECT
        c.segment,
        COUNT(DISTINCT c.id) as customer_count,
        COUNT(o.order_id) as order_count,
        SUM(o.amount) as total_revenue,
        AVG(o.amount) as avg_order_value
      FROM customers c
      JOIN orders o ON c.id = o.customer_id
      GROUP BY c.segment
      ORDER BY total_revenue DESC
    `);

    return {
      segmentAnalysis: result.rows
    };
  }
}
```

## Using NPM Packages

```typescript
import { parse } from 'csv-parse/sync';

export default {
  async run(ctx, parameters) {
    // Read CSV file
    const csvContent = await ctx.files.read(parameters.fileId);

    // Parse using csv-parse library
    const records = parse(csvContent, {
      columns: true,
      skip_empty_lines: true
    });

    // Load into DuckDB for SQL analysis
    await ctx.loadData('parsed_data', records);

    const summary = await ctx.query(`
      SELECT
        COUNT(*) as total_records,
        COUNT(DISTINCT column1) as unique_values
      FROM parsed_data
    `);

    return {
      summary: summary.rows[0],
      sample: records.slice(0, 10)
    };
  }
}
```

## Error Handling

```typescript
export default {
  async run(ctx, parameters) {
    try {
      // Validate parameters
      if (!parameters.datasource) {
        throw new Error('datasource parameter is required');
      }

      const datasource = ctx.getDatasource(parameters.datasource);
      if (!datasource) {
        throw new Error(`Datasource '${parameters.datasource}' not found`);
      }

      // Attempt query
      const result = await ctx.queryDatasource(
        parameters.datasource,
        parameters.query
      );

      return {
        success: true,
        rows: result.rows.length,
        data: result.rows
      };

    } catch (error) {
      ctx.log('Error occurred:', error.message);

      return {
        success: false,
        error: error.message,
        recoverable: error.message.includes('timeout')
      };
    }
  }
}
```

## Metadata and State

```typescript
export default {
  async run(ctx, parameters) {
    // Store metadata
    ctx.setMetadata('lastRunDate', new Date().toISOString());
    ctx.setMetadata('runCount', (ctx.getMetadata('runCount') || 0) + 1);

    // Get previous run data
    const lastRun = ctx.getMetadata('lastRunDate');
    const runCount = ctx.getMetadata('runCount');

    ctx.log(`This is run #${runCount}, last run: ${lastRun}`);

    // Perform analysis
    const result = await ctx.query(`
      SELECT COUNT(*) as count FROM my_table
    `);

    return {
      currentRun: {
        timestamp: new Date().toISOString(),
        runNumber: runCount
      },
      result: result.rows[0]
    };
  }
}
```

## Streaming Large Results

When dealing with large datasets, return aggregated results instead of raw data:

```typescript
export default {
  async run(ctx, parameters) {
    // Query large dataset
    const result = await ctx.queryDatasource(
      'big_data_warehouse',
      'SELECT * FROM transactions WHERE date >= ?',
      [parameters.since]
    );

    ctx.log(`Loaded ${result.rows.length} transactions`);

    // Don't return all rows! Aggregate instead
    await ctx.loadData('transactions', result.rows);

    const summary = await ctx.query(`
      SELECT
        DATE_TRUNC('day', date) as day,
        COUNT(*) as tx_count,
        SUM(amount) as total_amount,
        MIN(amount) as min_amount,
        MAX(amount) as max_amount,
        AVG(amount) as avg_amount
      FROM transactions
      GROUP BY day
      ORDER BY day
    `);

    // Return aggregated data (much smaller)
    return {
      dailySummary: summary.rows,
      totalTransactions: result.rows.length
    };
  }
}
```
