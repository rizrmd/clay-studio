export default {
    title: "File Analysis Example",
    
    description: "Demonstrates how to use file operations in analysis scripts",
    
    parameters: {
        file_id: {
            type: "string",
            required: false,
            description: "Optional file ID to analyze (if not provided, will list all files)"
        }
    },
    
    run: async function(ctx, params) {
        console.log("Starting file analysis...");
        
        // Example 1: List all uploaded files
        console.log("Listing all uploaded files...");
        const allFiles = await ctx.files.list();
        console.log(`Found ${allFiles.length} files:`);
        
        // Display file information
        for (const file of allFiles) {
            console.log(`- ${file.name} (${file.type}, ${file.size} bytes)`);
        }
        
        let targetFileId = params.file_id;
        
        // If no specific file is provided, use the first file
        if (!targetFileId && allFiles.length > 0) {
            targetFileId = allFiles[0].id;
            console.log(`\nUsing first file: ${allFiles[0].name}`);
        }
        
        if (targetFileId) {
            // Example 2: Get file metadata
            console.log(`\nGetting metadata for file ${targetFileId}...`);
            const metadata = await ctx.files.getMetadata(targetFileId);
            console.log("File metadata:", metadata);
            
            // Example 3: Read file content
            console.log(`\nReading content of file ${targetFileId}...`);
            try {
                const content = await ctx.files.read(targetFileId);
                console.log("File content (first 200 chars):", content.substring(0, 200));
                
                // Example 4: If it's a text file, search within it
                if (metadata.mime_type?.startsWith("text/") || 
                    metadata.name?.endsWith(".csv") || 
                    metadata.name?.endsWith(".json") ||
                    metadata.name?.endsWith(".txt")) {
                    
                    console.log("\nSearching for 'data' in file content...");
                    const searchResults = await ctx.files.searchContent(targetFileId, "data");
                    console.log("Search results:", searchResults);
                }
                
                // Example 5: Get a range of lines (for text files)
                if (metadata.mime_type?.startsWith("text/") || metadata.name?.endsWith(".txt")) {
                    console.log("\nGetting lines 1-5 from file...");
                    const rangeContent = await ctx.files.range(targetFileId, 1, 5);
                    console.log("Lines 1-5:", rangeContent);
                }
                
            } catch (error) {
                console.log("Error reading file:", error.message);
                
                // Try peeking instead for binary files
                console.log("Trying to peek at file content...");
                const peekContent = await ctx.files.peek(targetFileId, {
                    strategy: "head",
                    sample_size: 1000
                });
                console.log("Peek result:", peekContent);
            }
            
            // Example 6: Get download URL
            const downloadUrl = await ctx.files.getDownloadUrl(targetFileId);
            console.log(`\nDownload URL: ${downloadUrl}`);
        }
        
        // Example 7: Search for files
        if (allFiles.length > 0) {
            console.log("\nSearching for files containing 'data'...");
            const searchResults = await ctx.files.search("data");
            console.log(`Found ${searchResults.length} matching files:`);
            searchResults.forEach(file => {
                console.log(`- ${file.name}: ${file.snippet}`);
            });
        }
        
        return {
            status: "completed",
            message: "File analysis completed successfully",
            total_files_found: allFiles.length,
            analyzed_file_id: targetFileId,
            timestamp: new Date().toISOString()
        };
    }
}