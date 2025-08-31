# DataSource Update Improvements

## Overview
The `datasource_update` function has been enhanced to automatically test the connection after updating and use the `ask_user` interaction tool when the test fails.

## Key Changes

### Automatic Connection Testing
- After updating the datasource configuration, the function now automatically tests the connection
- If successful, it marks the datasource as active and updates `last_tested_at`
- Attempts to fetch and store schema information

### Interactive Error Handling (for interaction server type)
When the connection test fails and the server type is "interaction":
- Uses the `ask_user` tool to present options to the user
- Provides detailed error information
- Offers choices:
  - Retry with different credentials
  - Revert to previous configuration  
  - Keep changes (mark as inactive)
  - View connection details

### Non-Interactive Error Handling
When not on an interaction server:
- Returns a warning message indicating the update succeeded but connection failed
- Marks the datasource as inactive
- Provides error details in the response

## Benefits
1. **Immediate Feedback**: Users know right away if their updated configuration works
2. **Interactive Recovery**: When on an interaction server, users can immediately choose how to handle failures
3. **Better UX**: No need to manually run `datasource_test` after every update
4. **Automatic Schema Updates**: Successful connections automatically update schema information

## Code Location
- File: `backend/src/core/mcp/handlers.rs`
- Function: `datasource_update` (lines 927-1203)

## Usage Flow
1. User calls `datasource_update` with new configuration
2. System updates the database record
3. System automatically tests the new connection
4. If successful:
   - Marks datasource as active
   - Updates last_tested_at timestamp
   - Fetches and stores schema information
   - Returns success message with connection status
5. If failed (on interaction server):
   - Presents interactive options via ask_user
   - User can choose next action
6. If failed (non-interaction server):
   - Returns warning with error details
   - Datasource remains inactive