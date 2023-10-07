/// Makes sure the database path is valid.
pub fn validate_database_file_path(input: &str) -> String {
    let mut result = String::from(input);

    if input.ends_with('/') {
        // It's a directory path, so append "db.qkv" to it
        result.push_str("db.qkv");
    } else if !input.contains('.') {
        // It doesn't have an extension, so add ".qkv"
        result.push_str(".qkv");
    } else if !input.ends_with(".qkv") {
        // Ensure it ends with ".qkv"
        let index = input.rfind('.').unwrap_or(0);
        result.replace_range(index.., ".qkv");
    }

    result
}
