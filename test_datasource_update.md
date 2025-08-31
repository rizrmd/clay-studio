# Testing datasource_update MCP Tool

The `datasource_update` tool has been successfully implemented and added to the MCP handlers.

## Implementation Details

### Location
- File: `backend/src/core/mcp/handlers.rs`
- Function: `async fn datasource_update(...)`
- Added to `handle_tools_call` switch statement

### Functionality
The `datasource_update` function allows updating existing data source configurations:

#### Supported Parameters:
- `datasource_id` (required): The ID of the data source to update
- `name`: Update the data source name
- `host`: Update the host address
- `port`: Update the port number
- `database`: Update the database name
- `username`: Update the username
- `password`: Update the password
- `schema`: Update the schema
- `additional_params`: Object containing any additional source-specific parameters

### Behavior
1. Validates the datasource_id exists for the current project
2. Fetches current configuration from database
3. Merges provided updates with existing configuration
4. Updates the data source in the database
5. Marks connection as inactive (requires re-testing)
6. Returns success message with summary of updated fields

### Usage Examples

Update host and port:
```
datasource_update datasource_id="<id>" host="new-host.com" port=5432
```

Update credentials:
```
datasource_update datasource_id="<id>" username="new_user" password="new_password"
```

Update schema:
```
datasource_update datasource_id="<id>" schema="new_schema"
```

Multiple updates at once:
```
datasource_update datasource_id="<id>" host="new-host.com" database="new_db" username="new_user" schema="new_schema"
```

## Testing Steps

1. First list existing data sources:
   ```
   datasource_list
   ```

2. Update a data source (use an actual ID from the list):
   ```
   datasource_update datasource_id="<actual-id>" schema="test_schema"
   ```

3. Verify the update:
   ```
   datasource_detail datasource_id="<actual-id>"
   ```

4. Test the updated connection:
   ```
   datasource_test datasource_id="<actual-id>"
   ```

## Error Handling

The function handles the following error cases:
- Missing datasource_id parameter
- Non-existent datasource_id
- Database connection errors
- Invalid project association

## Notes

- After updating, the connection is marked as inactive
- Users should run `datasource_test` to verify the new configuration
- The function preserves existing configuration values not specified in the update
- Password updates are logged as "password (hidden)" for security