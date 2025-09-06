use std::fs;

fn main() {
    let claude_md_path = "/Users/riz/Developer/clay-studio/.clients/9f804a97-6b62-4e27-92ea-67ca00804f80/4f19b140-6ab9-4344-8931-62cda584c1a5/CLAUDE.md";
    
    match fs::read_to_string(&claude_md_path) {
        Ok(content) => {
            let has_validation = content.contains("MCP Interaction Parameter Validation");
            let has_breaking_change = content.contains("BREAKING CHANGE: show_table Parameter Format");
            
            println!("File exists");
            println!("Has validation section: {}", has_validation);
            println!("Has breaking change section: {}", has_breaking_change);
            println!("Should update: {}", !has_validation || !has_breaking_change);
        }
        Err(e) => {
            println!("Cannot read file: {}", e);
        }
    }
}
