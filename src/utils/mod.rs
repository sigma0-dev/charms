pub(crate) mod logger;

/// Create a string representation of the index `i` in the format `$xxxx`.
pub fn str_index(i: &usize) -> String {
    format!("${:04}", i)
}
