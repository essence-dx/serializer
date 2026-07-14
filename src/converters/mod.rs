//! Converters from other formats to DX format
//!
//! Supports: JSON, YAML, TOON, TOML → DX
//!
//! All converters apply optimization automatically:
//! - Abbreviated keys (name → n, version → v)
//! - Minimal prefixes (context → c, media → m)
//! - Inline chaining with ^
//! - Compact arrays with |
//! - 2-letter language codes

// JSON, YAML, TOML converters require the converters feature (serde dependencies)
#[cfg(feature = "converters")]
/// JSON to DX conversion support.
pub mod json;
#[cfg(feature = "converters")]
/// TOML to DX conversion support.
pub mod toml;
#[cfg(feature = "converters")]
/// YAML to DX conversion support.
pub mod yaml;

// TOON converter has no external dependencies
/// TOON to DX conversion support.
pub mod toon;

// Property tests for converters require the converters feature
#[cfg(all(test, feature = "converters"))]
mod converter_props;

#[cfg(feature = "converters")]
pub use json::{dx_to_json, dx_to_json_doc, dx_to_json_min, json_to_document, json_to_dx};
#[cfg(feature = "converters")]
pub use toml::{dx_to_toml, dx_to_toml_doc, toml_to_document, toml_to_dx};
pub use toon::{dx_to_toon, toon_to_dx};
#[cfg(feature = "converters")]
pub use yaml::{dx_to_yaml, dx_to_yaml_doc, yaml_to_document, yaml_to_dx};

/// Convert any supported format to DX
#[cfg(feature = "converters")]
pub fn convert_to_dx(input: &str, format: &str) -> Result<String, String> {
    match format.to_lowercase().as_str() {
        "json" => json_to_dx(input),
        "yaml" | "yml" => yaml_to_dx(input),
        "toon" => toon_to_dx(input),
        "toml" => toml_to_dx(input),
        _ => Err(format!("Unsupported format: {format}")),
    }
}

/// Convert any supported format to DX (minimal version without converters feature)
#[cfg(not(feature = "converters"))]
pub fn convert_to_dx(input: &str, format: &str) -> Result<String, String> {
    match format.to_lowercase().as_str() {
        "toon" => toon_to_dx(input),
        _ => Err(format!(
            "Format '{}' requires the 'converters' feature. Only 'toon' is available without it.",
            format
        )),
    }
}
