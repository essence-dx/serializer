//! Human-readable formatter for DX format
//!
//! This module is currently a placeholder. The original format_human.rs
//! was dead code (never compiled) and referenced types (`DxData`) that
//! no longer exist in this codebase.

#[cfg(test)]
mod tests {
    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().chain(chars).collect(),
        }
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize_first("name"), "Name");
        assert_eq!(capitalize_first("id"), "Id");
        assert_eq!(capitalize_first(""), "");
    }
}
