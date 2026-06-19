//! DX-native Biome configuration parsed from the extensionless root `dx` file.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::llm::ConvertError;
use crate::{DxDocument, DxLlmValue, llm_to_document};

const BIOME_SECTION: &str = "biome";
const TARGET_COLUMN: &str = "target";
const PATH_COLUMN: &str = "path";
const ENABLED_COLUMN: &str = "enabled";
const DEFAULT_PATH: &str = ".";

/// Biome target class declared in DX configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxBiomeTarget {
    /// Applies to both lint and format adapters.
    All,
    /// Applies only to Biome lint.
    Lint,
    /// Applies only to Biome format.
    Format,
}

impl DxBiomeTarget {
    /// Returns true when this declaration applies to the requested target.
    #[must_use]
    pub fn includes(self, requested: Self) -> bool {
        self == Self::All || self == requested
    }
}

/// Parsed Biome configuration from a DX document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxBiomeConfig {
    entries: Vec<DxBiomeConfigEntry>,
}

impl DxBiomeConfig {
    /// Returns all enabled Biome entries in source order.
    #[must_use]
    pub fn entries(&self) -> &[DxBiomeConfigEntry] {
        &self.entries
    }

    /// Returns true when the requested Biome target has at least one enabled path.
    #[must_use]
    pub fn is_enabled_for(&self, target: DxBiomeTarget) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.target.includes(target))
    }

    /// Returns deduplicated paths for the requested target in source order.
    #[must_use]
    pub fn paths_for(&self, target: DxBiomeTarget) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut paths = Vec::new();
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.target.includes(target))
        {
            if seen.insert(entry.path.clone()) {
                paths.push(entry.path.clone());
            }
        }
        paths
    }
}

/// One enabled Biome target/path row from DX configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxBiomeConfigEntry {
    /// Target class for this row.
    pub target: DxBiomeTarget,
    /// Path passed to the Biome CLI for this row.
    pub path: String,
}

/// Errors produced while reading or validating DX Biome configuration.
#[derive(Debug, Error)]
pub enum DxBiomeConfigError {
    /// The extensionless `dx` file could not be read.
    #[error("failed to read Biome dx config at {path}: {source}")]
    Read {
        /// Display path for the failed read.
        path: String,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// The extensionless `dx` file could not be parsed as a DX document.
    #[error("failed to parse Biome dx config: {0}")]
    Parse(#[from] ConvertError),
    /// The `biome[...]` table is missing a required column.
    #[error("biome table must include a {column} column")]
    MissingColumn {
        /// Missing column name.
        column: &'static str,
    },
    /// A row declared an unsupported Biome target.
    #[error("invalid Biome target at row {row}: {value}")]
    InvalidTarget {
        /// One-based table row number.
        row: usize,
        /// Unsupported target value.
        value: String,
    },
    /// A row declared an unsupported `enabled` value.
    #[error("invalid Biome enabled value at row {row}: {value}")]
    InvalidEnabled {
        /// One-based table row number.
        row: usize,
        /// Unsupported enabled value.
        value: String,
    },
    /// A row declared an unsafe or invalid Biome path.
    #[error("invalid Biome path at row {row}: {value} ({reason})")]
    InvalidPath {
        /// One-based table row number.
        row: usize,
        /// Invalid path value.
        value: String,
        /// Validation reason.
        reason: &'static str,
    },
}

/// Loads DX Biome configuration from an extensionless `dx` file path.
///
/// # Errors
///
/// Returns an error when the file cannot be read, the DX document cannot be
/// parsed, or the `biome[...]` table contains invalid values.
pub fn load_biome_config(
    path: impl AsRef<Path>,
) -> Result<Option<DxBiomeConfig>, DxBiomeConfigError> {
    let path = path.as_ref();
    if !path.is_file() {
        return Ok(None);
    }

    let source = fs::read_to_string(path).map_err(|source| DxBiomeConfigError::Read {
        path: path.display().to_string(),
        source,
    })?;
    biome_config_from_source(&source)
}

/// Parses DX Biome configuration directly from extensionless `dx` source text.
///
/// # Errors
///
/// Returns an error when the source is not valid DX text or the `biome[...]`
/// table contains invalid values.
pub fn biome_config_from_source(source: &str) -> Result<Option<DxBiomeConfig>, DxBiomeConfigError> {
    let document = llm_to_document(source)?;
    biome_config_from_document(&document)
}

/// Extracts DX Biome configuration from a parsed DX document.
///
/// # Errors
///
/// Returns an error when the `biome[...]` table is present but malformed.
pub fn biome_config_from_document(
    document: &DxDocument,
) -> Result<Option<DxBiomeConfig>, DxBiomeConfigError> {
    let Some(section) = document.section_by_name(BIOME_SECTION) else {
        return Ok(None);
    };

    let target_index =
        section
            .column_index(TARGET_COLUMN)
            .ok_or(DxBiomeConfigError::MissingColumn {
                column: TARGET_COLUMN,
            })?;
    let path_index = section.column_index(PATH_COLUMN);
    let enabled_index = section.column_index(ENABLED_COLUMN);
    let mut entries = Vec::new();

    for (index, row) in section.rows.iter().enumerate() {
        let row_number = index + 1;
        if !enabled_value(row, enabled_index, row_number)? {
            continue;
        }

        let target_value = row
            .get(target_index)
            .map(cell_text)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| DxBiomeConfigError::InvalidTarget {
                row: row_number,
                value: String::new(),
            })?;
        let target =
            parse_target(&target_value).ok_or_else(|| DxBiomeConfigError::InvalidTarget {
                row: row_number,
                value: target_value.clone(),
            })?;

        let path = path_value(row, path_index, row_number)?;
        validate_path(&path, row_number)?;
        entries.push(DxBiomeConfigEntry {
            target,
            path: normalize_relative_path(&path),
        });
    }

    Ok(Some(DxBiomeConfig { entries }))
}

fn enabled_value(
    row: &[DxLlmValue],
    enabled_index: Option<usize>,
    row_number: usize,
) -> Result<bool, DxBiomeConfigError> {
    let Some(index) = enabled_index else {
        return Ok(true);
    };
    let Some(value) = row.get(index) else {
        return Ok(true);
    };

    match value {
        DxLlmValue::Bool(value) => Ok(*value),
        DxLlmValue::Num(value) if value.abs() < f64::EPSILON => Ok(false),
        DxLlmValue::Num(value) if (*value - 1.0).abs() < f64::EPSILON => Ok(true),
        DxLlmValue::Str(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" | "1" | "enabled" => Ok(true),
            "false" | "no" | "off" | "0" | "disabled" => Ok(false),
            _ => Err(DxBiomeConfigError::InvalidEnabled {
                row: row_number,
                value: value.clone(),
            }),
        },
        DxLlmValue::Null => Ok(true),
        _ => Err(DxBiomeConfigError::InvalidEnabled {
            row: row_number,
            value: value.to_string(),
        }),
    }
}

fn parse_target(value: &str) -> Option<DxBiomeTarget> {
    match value.trim().to_ascii_lowercase().as_str() {
        "all" | "check" | "lint-format" | "lint_format" | "lint+format" => Some(DxBiomeTarget::All),
        "lint" => Some(DxBiomeTarget::Lint),
        "format" | "formatter" => Some(DxBiomeTarget::Format),
        _ => None,
    }
}

fn cell_text(value: &DxLlmValue) -> String {
    value.to_string()
}

fn path_value(
    row: &[DxLlmValue],
    path_index: Option<usize>,
    row_number: usize,
) -> Result<String, DxBiomeConfigError> {
    let Some(index) = path_index else {
        return Ok(DEFAULT_PATH.to_string());
    };
    let Some(value) = row.get(index) else {
        return Ok(DEFAULT_PATH.to_string());
    };
    match value {
        DxLlmValue::Str(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(DxBiomeConfigError::InvalidPath {
                    row: row_number,
                    value: value.clone(),
                    reason: "path cannot be empty",
                });
            }
            Ok(trimmed.to_string())
        }
        _ => Err(DxBiomeConfigError::InvalidPath {
            row: row_number,
            value: value.to_string(),
            reason: "path must be a string",
        }),
    }
}

fn validate_path(path: &str, row_number: usize) -> Result<(), DxBiomeConfigError> {
    if path.trim().is_empty() {
        return Err(DxBiomeConfigError::InvalidPath {
            row: row_number,
            value: path.to_string(),
            reason: "path cannot be empty",
        });
    }
    if path.starts_with('-') {
        return Err(DxBiomeConfigError::InvalidPath {
            row: row_number,
            value: path.to_string(),
            reason: "path cannot look like a command-line option",
        });
    }
    let normalized = path.replace('\\', "/");
    if is_absolute_like(&normalized) {
        return Err(DxBiomeConfigError::InvalidPath {
            row: row_number,
            value: path.to_string(),
            reason: "path must be relative to the project root",
        });
    }
    if normalized.split('/').any(|component| component == "..") {
        return Err(DxBiomeConfigError::InvalidPath {
            row: row_number,
            value: path.to_string(),
            reason: "path cannot escape the project root",
        });
    }
    if path.chars().any(char::is_control) {
        return Err(DxBiomeConfigError::InvalidPath {
            row: row_number,
            value: path.to_string(),
            reason: "path cannot contain control characters",
        });
    }
    Ok(())
}

fn is_absolute_like(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with("//")
        || path
            .as_bytes()
            .get(1)
            .is_some_and(|separator| *separator == b':')
}

fn normalize_relative_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let parts = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    if parts.is_empty() {
        DEFAULT_PATH.to_string()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_biome_config_from_extensionless_dx_source() {
        let source = r"
project(name=demo kind=dx-project)

biome[target path enabled](
lint src true
format tests true
all packages/app true
)
";

        let config = biome_config_from_source(source).unwrap().unwrap();

        assert_eq!(
            config.paths_for(DxBiomeTarget::Lint),
            vec!["src".to_string(), "packages/app".to_string()]
        );
        assert_eq!(
            config.paths_for(DxBiomeTarget::Format),
            vec!["tests".to_string(), "packages/app".to_string()]
        );
    }

    #[test]
    fn missing_biome_table_returns_none() {
        let config = biome_config_from_source("project(name=demo kind=dx-project)").unwrap();

        assert!(config.is_none());
    }

    #[test]
    fn rejects_paths_that_would_be_cli_options() {
        let error = biome_config_from_source(
            r#"
biome[target path](
lint "--write"
)
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("--write"));
    }

    #[test]
    fn trims_and_deduplicates_biome_paths_in_source_order() {
        let config = biome_config_from_source(
            r#"
biome[target path](
lint " src "
lint src
lint ./src
lint src/.
lint tests
)
"#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            config.paths_for(DxBiomeTarget::Lint),
            vec!["src".to_string(), "tests".to_string()]
        );
    }

    #[test]
    fn rejects_absolute_and_parent_traversal_biome_paths() {
        for path in [
            "/tmp/project",
            "C:/repo/src",
            "//server/share",
            "../outside",
        ] {
            let error = biome_config_from_source(&format!(
                r#"
biome[target path](
lint "{path}"
)
"#
            ))
            .unwrap_err();

            assert!(
                error.to_string().contains(path),
                "error should mention rejected path {path}: {error}"
            );
        }

        for path in ["..\\outside", "src\\..\\outside", "C:\\repo\\src"] {
            assert!(
                validate_path(path, 1).is_err(),
                "path should be rejected independent of host OS: {path}"
            );
        }
    }

    #[test]
    fn rejects_non_scalar_biome_paths() {
        let mut document = DxDocument::new();
        let mut section = crate::DxSection::new(vec!["target".to_string(), "path".to_string()]);
        section
            .add_row(vec![
                DxLlmValue::Str("lint".to_string()),
                DxLlmValue::Arr(vec![DxLlmValue::Str("src".to_string())]),
            ])
            .unwrap();
        document.sections.insert('b', section);
        document.section_names.insert('b', "biome".to_string());

        let error = biome_config_from_document(&document).unwrap_err();

        assert!(error.to_string().contains("path"));
    }

    #[test]
    fn disabled_rows_do_not_validate_draft_target_or_path() {
        let config = biome_config_from_source(
            r#"
biome[target path enabled](
future "--write" false
lint src true
)
"#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            config.paths_for(DxBiomeTarget::Lint),
            vec!["src".to_string()]
        );
    }
}
