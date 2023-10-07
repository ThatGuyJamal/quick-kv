/// Makes sure the database path is valid.
pub fn validate_database_file_path(input: &str) -> String
{
    if input.ends_with(".qkv") {
        input.to_string()
    } else if let Some(index) = input.rfind('.') {
        format!("{}.qkv", &input[..index])
    } else {
        format!("{}.qkv", input)
    }
}
