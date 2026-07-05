use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

/// Env var to override the DX workspace root.
const DX_HOME_ENV: &str = "DX_HOME";

/// Errors that can occur during config loading.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read dx config at {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse dx config at {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("{var} is set but empty")]
    EmptyEnvVar { var: String },
    #[error("{var} must be an absolute path, got {value}")]
    RelativeEnvVar { var: String, value: String },
}

/// The root dx workspace configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DxConfig {
    pub workspace: WorkspaceConfig,
    pub paths: PathsConfig,
    pub tools: ToolsConfig,
}

impl DxConfig {
    /// Discover and load the nearest `dx` config, walking up from `cwd`.
    pub fn load(cwd: &Path) -> Result<Self, ConfigError> {
        Self::load_with_home(cwd, env::var_os(DX_HOME_ENV))
    }

    /// Load config with an optional DX_HOME override.
    pub fn load_with_home(cwd: &Path, dx_home: Option<OsString>) -> Result<Self, ConfigError> {
        if let Some(home) = dx_home_override_root(dx_home)? {
            let path = home.join("dx");
            if path.is_file() {
                return Self::from_path(&path);
            }
            return Ok(Self::with_root(&home));
        }

        let Some(path) = discover_config_path(cwd) else {
            return Ok(Self::with_root(cwd));
        };

        Self::from_path(&path)
    }

    /// Load from a specific dx file path.
    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError { path: path.to_path_buf(), source: e })?;
        let mut config: Self = toml::from_str(&source)
            .map_err(|e| ConfigError::ParseError { path: path.to_path_buf(), source: e })?;
        let base = path.parent().unwrap_or(Path::new("."));
        config.absolutize(base);
        Ok(config)
    }

    /// Create a default config rooted at the given workspace root.
    pub fn with_root(root: &Path) -> Self {
        let mut config = Self::default();
        config.workspace.root = root.to_path_buf();
        config.paths = PathsConfig::default_with_root(root);
        config
    }

    /// Return the DX home directory root.
    ///
    /// Uses `paths.dx_home` from config if set, otherwise:
    /// Windows: `%LOCALAPPDATA%/dx`
    /// macOS:   `~/Library/Application Support/dx`
    /// Linux:   `$XDG_DATA_HOME/dx` or `~/.local/share/dx`
    ///
    /// Always returns a non-empty path — if config has no value, the OS default is returned.
    pub fn dx_home_dir(&self) -> PathBuf {
        if !self.paths.dx_home.as_os_str().is_empty() {
            self.paths.dx_home.clone()
        } else {
            resolve_dx_home_dir()
        }
    }

    /// Return the binaries directory (under DX home).
    pub fn bin_dir(&self) -> PathBuf {
        self.dx_home_dir().join("bin")
    }

    /// Return the global cache directory (from config or OS default).
    pub fn global_cache_dir(&self) -> PathBuf {
        if !self.paths.global_cache.as_os_str().is_empty() {
            self.paths.global_cache.clone()
        } else {
            self.dx_home_dir().join("cache")
        }
    }

    /// Alias for `global_cache_dir`.
    pub fn cache_dir(&self) -> PathBuf {
        self.global_cache_dir()
    }

    /// Return the user config directory (under DX home).
    pub fn config_dir(&self) -> PathBuf {
        self.dx_home_dir().join("config")
    }

    /// Return the application data directory (under DX home).
    pub fn data_dir(&self) -> PathBuf {
        self.dx_home_dir().join("data")
    }

    fn absolutize(&mut self, base: &Path) {
        let root = absolutize(base, &self.workspace.root);
        self.workspace.root = root.clone();
        self.paths.absolutize(&root);
    }
}

/// Workspace identity.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub name: String,
    pub root: PathBuf,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            name: "DX".to_string(),
            root: PathBuf::from("."),
        }
    }
}

/// Paths to all DX tool directories.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub cli: PathBuf,
    pub www: PathBuf,
    pub website: PathBuf,
    pub forge: PathBuf,
    pub check: PathBuf,
    pub style: PathBuf,
    pub js: PathBuf,
    pub build: PathBuf,
    #[serde(alias = "py")]
    pub python: PathBuf,
    pub native: PathBuf,
    pub icon: PathBuf,
    pub media: PathBuf,
    pub serializer: PathBuf,
    #[serde(alias = "py_package_manager")]
    pub python_package_manager: PathBuf,
    pub dx_agents: PathBuf,
    pub inspirations: PathBuf,
    pub cache: PathBuf,
    pub dx_home: PathBuf,
    pub global_cache: PathBuf,
    pub cargo_home: PathBuf,
}

impl PathsConfig {
    fn default_with_root(root: &Path) -> Self {
        let dx_home = resolve_dx_home_dir();
        Self {
            cli: root.join("cli"),
            www: root.join("www"),
            website: root.join("website"),
            forge: root.join("forge"),
            check: root.join("check"),
            style: root.join("style"),
            js: root.join("js"),
            build: root.join("build"),
            python: root.join("py"),
            native: root.join("native"),
            icon: root.join("icon"),
            media: root.join("media"),
            serializer: root.join("serializer"),
            python_package_manager: root.join("py").join("package-manager"),
            dx_agents: root.join("agent"),
            inspirations: root.join("inspirations"),
            cache: root.join(".dx").join("cache"),
            dx_home: PathBuf::new(),
            global_cache: dx_home.join("cache"),
            cargo_home: root.join("cli").join(".cargo-home"),
        }
    }

    fn absolutize(&mut self, root: &Path) {
        self.cli = absolutize(root, &self.cli);
        self.www = absolutize(root, &self.www);
        self.website = absolutize(root, &self.website);
        self.forge = absolutize(root, &self.forge);
        self.check = absolutize(root, &self.check);
        self.style = absolutize(root, &self.style);
        self.js = absolutize(root, &self.js);
        self.build = absolutize(root, &self.build);
        self.python = absolutize(root, &self.python);
        self.native = absolutize(root, &self.native);
        self.icon = absolutize(root, &self.icon);
        self.media = absolutize(root, &self.media);
        self.serializer = absolutize(root, &self.serializer);
        self.python_package_manager = absolutize(root, &self.python_package_manager);
        self.dx_agents = absolutize(root, &self.dx_agents);
        self.inspirations = absolutize(root, &self.inspirations);
        self.cache = absolutize(root, &self.cache);
        self.dx_home = absolutize_optional(root, &self.dx_home, resolve_dx_home_dir);
        if self.global_cache.as_os_str().is_empty() {
            self.global_cache = self.dx_home.join("cache");
        } else if !self.global_cache.is_absolute() {
            self.global_cache = root.join(&self.global_cache);
        }
        self.cargo_home = absolutize(root, &self.cargo_home);
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self::default_with_root(Path::new("."))
    }
}

/// Tool-level configuration paths.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ToolsConfig {
    pub scoop_root: PathBuf,
    pub local_bin: PathBuf,
    pub ffmpeg_dev: PathBuf,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            scoop_root: PathBuf::from("G:/Dev/Tools/Scoop"),
            local_bin: PathBuf::from("cli/tools/bin"),
            ffmpeg_dev: PathBuf::from("cli/tools/ffmpeg-dev/ffmpeg-8.1.1-full_build-shared"),
        }
    }
}

/// Walk up from `root` looking for an extensionless `dx` config file.
/// Skips files that look like Serializer project configs (containing `project(...)`
/// or Serializer table sections `name[col](...)`).
fn discover_config_path(root: &Path) -> Option<PathBuf> {
    for ancestor in root.ancestors() {
        let candidate = ancestor.join("dx");
        if candidate.is_file() && !looks_like_project_config(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

/// Absolutize a path, but only if it's non-empty (empty = use fallback).
fn absolutize_optional(root: &Path, path: &Path, fallback: fn() -> PathBuf) -> PathBuf {
    if path.as_os_str().is_empty() {
        fallback()
    } else if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

/// Resolve the OS default DX home directory.
///
/// This is the root for all DX global state (binaries, cache, config, data).
/// Windows: `%LOCALAPPDATA%/dx`
/// macOS:   `~/Library/Application Support/dx`
/// Linux:   `$XDG_DATA_HOME/dx` or `~/.local/share/dx`
fn resolve_dx_home_dir() -> PathBuf {
    if let Some(base) = dirs::data_dir() {
        base.join("dx")
    } else {
        PathBuf::from("~/.local/share/dx")
    }
}

fn dx_home_override_root(raw: Option<OsString>) -> Result<Option<PathBuf>, ConfigError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.is_empty() {
        return Err(ConfigError::EmptyEnvVar { var: DX_HOME_ENV.to_string() });
    }
    let root = PathBuf::from(raw);
    if !root.is_absolute() {
        return Err(ConfigError::RelativeEnvVar {
            var: DX_HOME_ENV.to_string(),
            value: root.display().to_string(),
        });
    }
    Ok(Some(root))
}

/// Write a `.sr` file in DX LLM format (key=value pairs).
///
/// The serializer daemon (`dx-sr-watch`) will auto-compile `.sr` -> `.machine`.
/// Call this to persist tool state for fast runtime loading.
///
/// # Example
/// ```no_run
/// use dx_config::write_sr_file;
/// write_sr_file(".dx/serializer/forge-cache.sr", &[
///     ("name", "forge"),
///     ("version", "1.0.0"),
///     ("status", "ready"),
/// ]).unwrap();
/// ```
pub fn write_sr_file(path: impl AsRef<Path>, entries: &[(&str, &str)]) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf: Vec<u8> = Vec::new();
    for (key, value) in entries {
        write!(buf, "{key}=")?;
        write_llm_value(&mut buf, value)?;
        buf.push(b'\n');
    }
    // Atomic write via temp file + rename
    let tmp = path.with_extension("sr.tmp");
    fs::write(&tmp, &buf)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Read a `.sr` file as key-value pairs.
///
/// Parses the DX LLM format (key=value lines, quoted values supported).
/// Returns `None` if the file doesn't exist or fails to parse.
///
/// # Example
/// ```no_run
/// use dx_config::read_sr_file;
/// if let Some(entries) = read_sr_file(".dx/serializer/forge-cache.sr") {
///     for (key, value) in &entries {
///         println!("{key} = {value}");
///     }
/// }
/// ```
pub fn read_sr_file(path: impl AsRef<Path>) -> Option<HashMap<String, String>> {
    let text = fs::read_to_string(path.as_ref()).ok()?;
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim();
            let unquoted = if (value.starts_with('"') && value.ends_with('"')) && value.len() >= 2 {
                &value[1..value.len() - 1]
            } else {
                value
            };
            map.insert(key, unquoted.to_string());
        }
    }
    Some(map)
}

/// Determine the `.machine` path for a given `.sr` path.
pub fn machine_path_from_sr(sr_path: impl AsRef<Path>) -> PathBuf {
    sr_path.as_ref().with_extension("machine")
}

/// Check if a `.machine` file is fresher than its `.sr` source by mtime.
#[must_use]
pub fn machine_is_fresher(machine_path: impl AsRef<Path>, sr_path: impl AsRef<Path>) -> bool {
    let machine_mtime = fs::metadata(machine_path.as_ref())
        .and_then(|m| m.modified())
        .ok();
    let sr_mtime = fs::metadata(sr_path.as_ref())
        .and_then(|m| m.modified())
        .ok();
    match (machine_mtime, sr_mtime) {
        (Some(mm), Some(sm)) => mm >= sm,
        _ => false,
    }
}

/// Try to read a `.machine` file (requires the `machine` feature).
///
/// Returns the key-value pairs from the compiled machine document.
/// If the `machine` feature is not enabled, returns `None`.
#[cfg(feature = "machine")]
pub fn read_machine_file(path: impl AsRef<Path>) -> Option<HashMap<String, String>> {
    use serializer::machine_bytes_to_document;
    let bytes = fs::read(path.as_ref()).ok()?;
    let doc = machine_bytes_to_document(&bytes).ok()?;
    let mut map = HashMap::new();
    for (key, value) in &doc.context {
        map.insert(key.clone(), value.to_string());
    }
    Some(map)
}

/// Non-machined version that always returns `None` (placeholder for when feature is off).
#[cfg(not(feature = "machine"))]
pub fn read_machine_file(_path: impl AsRef<Path>) -> Option<HashMap<String, String>> {
    None
}

/// Try to read state from `.machine` (fast), falling back to `.sr` (slow).
///
/// If the `machine` feature is enabled and a fresh `.machine` file exists,
/// it will be parsed using the fast serializer machine format.
/// Otherwise, the `.sr` file is parsed as plain key-value text.
pub fn read_machine_or_sr(sr_path: impl AsRef<Path>) -> Option<HashMap<String, String>> {
    let sr_path = sr_path.as_ref();
    let machine_path = machine_path_from_sr(sr_path);

    // Fast path: try .machine if fresh
    if machine_path.exists() && machine_is_fresher(&machine_path, sr_path) {
        if let Some(entries) = read_machine_file(&machine_path) {
            return Some(entries);
        }
    }

    // Slow path: parse .sr as LLM text
    read_sr_file(sr_path)
}

fn write_llm_value(buf: &mut Vec<u8>, value: &str) -> io::Result<()> {
    if value.is_empty() {
        buf.extend_from_slice(b"\"\"");
        return Ok(());
    }
    // Quote if contains spaces, quotes, brackets, or special chars
    let needs_quoting = value.contains(|c: char| {
        c.is_ascii_whitespace() || c == '"' || c == '[' || c == ']' || c == '=' || c == '#'
    });
    if needs_quoting {
        buf.push(b'"');
        for c in value.chars() {
            if c == '"' || c == '\\' {
                buf.push(b'\\');
            }
            let mut tmp = [0u8; 4];
            buf.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
        }
        buf.push(b'"');
    } else {
        buf.extend_from_slice(value.as_bytes());
    }
    Ok(())
}

fn looks_like_project_config(path: &Path) -> bool {
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    contains_serializer_project_entry(&source) || contains_serializer_table_section(&source)
}

fn contains_serializer_project_entry(source: &str) -> bool {
    source
        .lines()
        .map(|line| line.trim().trim_start_matches('\u{feff}'))
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .is_some_and(|line| {
            line.starts_with("project(")
                || line.starts_with("contract(")
                || line.starts_with("runtime(")
                || line.starts_with("www(")
        })
}

fn contains_serializer_table_section(source: &str) -> bool {
    let mut pos = 0;
    let bytes = source.as_bytes();
    while pos < source.len() {
        pos = skip_whitespace_and_comments(source, pos);
        if pos >= source.len() {
            return false;
        }
        if let Some(end) = parse_identifier(source, pos) {
            pos = end;
        } else {
            pos = skip_to_end_of_line(source, pos);
            continue;
        }
        pos = skip_whitespace_and_comments(source, pos);
        if bytes.get(pos) != Some(&b'[') {
            pos = skip_to_end_of_line(source, pos);
            continue;
        }
        pos += 1;
        pos = skip_schema_columns(source, pos);
        pos = skip_whitespace_and_comments(source, pos);
        if bytes.get(pos) == Some(&b'(') {
            return true;
        }
        pos = skip_to_end_of_line(source, pos);
    }
    false
}

fn skip_whitespace_and_comments(s: &str, mut pos: usize) -> usize {
    loop {
        pos = skip_whitespace(s, pos);
        if s.as_bytes().get(pos) == Some(&b'#') {
            pos = skip_to_end_of_line(s, pos);
            continue;
        }
        break;
    }
    pos
}

fn skip_whitespace(s: &str, mut pos: usize) -> usize {
    let bytes = s.as_bytes();
    while let Some(&b) = bytes.get(pos) {
        if b.is_ascii_whitespace() {
            pos += 1;
        } else {
            break;
        }
    }
    pos
}

fn skip_to_end_of_line(s: &str, mut pos: usize) -> usize {
    while let Some(b) = s.as_bytes().get(pos) {
        if *b == b'\n' || *b == b'\r' {
            pos += 1;
            break;
        }
        pos += 1;
    }
    pos
}

fn parse_identifier(s: &str, mut pos: usize) -> Option<usize> {
    let start = pos;
    while let Some(b) = s.as_bytes().get(pos) {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'@') {
            pos += 1;
        } else {
            break;
        }
    }
    (pos > start).then_some(pos)
}

fn skip_schema_columns(s: &str, mut pos: usize) -> usize {
    let mut in_quotes = false;
    let mut escape = false;
    while let Some(b) = s.as_bytes().get(pos) {
        if escape {
            escape = false;
            pos += 1;
            continue;
        }
        match b {
            b'\\' if in_quotes => escape = true,
            b'"' => in_quotes = !in_quotes,
            b']' if !in_quotes => return pos + 1,
            _ => {}
        }
        pos += 1;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_extensionless_dx() {
        let temp = tempfile_dir("dx-config-load");
        let config_path = temp.join("dx");
        fs::write(&config_path, r#"
workspace.name="DX"
workspace.root="G:/Dx"
paths.cli="cli"
paths.www="www"
paths.forge="forge"
paths.check="check"
paths.style="style"
paths.js="js"
paths.build="build"
paths.icon="icon"
paths.serializer="serializer"
"#).expect("write dx config");

        let config = DxConfig::from_path(&config_path).expect("load dx config");
        assert_eq!(config.workspace.root, PathBuf::from("G:/Dx"));
        assert_eq!(config.paths.cli, PathBuf::from("G:/Dx").join("cli"));
        assert_eq!(config.paths.www, PathBuf::from("G:/Dx").join("www"));
        assert_eq!(config.paths.forge, PathBuf::from("G:/Dx").join("forge"));
    }

    #[test]
    fn dx_home_returns_os_data_dir_by_default() {
        let config = DxConfig::default();
        let home = config.dx_home_dir();
        // On any OS, this should be non-empty and resolve to a path
        assert!(!home.as_os_str().is_empty());
        // Should end with "dx"
        assert_eq!(home.file_name().unwrap().to_str().unwrap(), "dx");
    }

    #[test]
    fn bin_dir_is_under_dx_home() {
        let config = DxConfig::default();
        let home = config.dx_home_dir();
        assert_eq!(config.bin_dir(), home.join("bin"));
    }

    #[test]
    fn config_dir_is_under_dx_home() {
        let config = DxConfig::default();
        let home = config.dx_home_dir();
        assert_eq!(config.config_dir(), home.join("config"));
    }

    #[test]
    fn data_dir_is_under_dx_home() {
        let config = DxConfig::default();
        let home = config.dx_home_dir();
        assert_eq!(config.data_dir(), home.join("data"));
    }

    #[test]
    fn cache_dir_defaults_to_dx_home_cache() {
        let config = DxConfig::default();
        let home = config.dx_home_dir();
        assert_eq!(config.global_cache_dir(), home.join("cache"));
        assert_eq!(config.cache_dir(), home.join("cache"));
    }

    #[test]
    fn dx_home_from_config_overrides_default() {
        let temp = tempfile_dir("dx-config-home-override");
        let custom_home = tempfile_dir("dx-custom-home");
        let config_path = temp.join("dx");
        let custom_str = custom_home.to_str().unwrap().replace('\\', "/");
        let content = format!(
            "workspace.name=\"DX\"\nworkspace.root=\".\"\npaths.dx_home=\"{custom_str}\"\n"
        );
        fs::write(&config_path, &content).expect("write dx config");
        // Verify the written content
        let read_back = fs::read_to_string(&config_path).expect("read back");
        assert!(read_back.contains(&custom_str), "written file should contain the custom path");

        let config = DxConfig::from_path(&config_path).expect("load dx config");
        // Check raw field before method calls
        eprintln!("paths.dx_home raw: {:?}", config.paths.dx_home);
        eprintln!("paths.global_cache raw: {:?}", config.paths.global_cache);
        eprintln!("dx_home_dir(): {:?}", config.dx_home_dir());
        eprintln!("global_cache_dir(): {:?}", config.global_cache_dir());
        assert_eq!(config.dx_home_dir(), custom_home);
        assert_eq!(config.bin_dir(), custom_home.join("bin"));
        assert_eq!(config.global_cache_dir(), custom_home.join("cache"));
    }

    #[test]
    fn global_cache_from_config_overrides_default() {
        let temp = tempfile_dir("dx-config-global-cache");
        let custom_cache = tempfile_dir("dx-custom-cache");
        let config_path = temp.join("dx");
        let cache_str = custom_cache.to_str().unwrap().replace('\\', "/");
        fs::write(&config_path, format!(r#"
workspace.name="DX"
workspace.root="."
paths.global_cache="{cache_str}"
"#)).expect("write dx config");

        let config = DxConfig::from_path(&config_path).expect("load dx config");
        assert_eq!(config.global_cache_dir(), custom_cache);
    }

    #[test]
    fn discovers_dx_from_parent() {
        let temp = tempfile_dir("dx-config-discover");
        let child = temp.join("website");
        fs::create_dir_all(&child).expect("create child");
        fs::write(temp.join("dx"), r#"
workspace.name="DX"
workspace.root="."
paths.cli="cli"
"#).expect("write dx config");

        let discovered = discover_config_path(&child).expect("discover dx config");
        assert_eq!(discovered, temp.join("dx"));
    }

    #[test]
    fn skips_project_serializer_config() {
        let temp = tempfile_dir("dx-config-skip-project");
        let project = temp.join("apps").join("demo");
        fs::create_dir_all(&project).expect("create project");
        fs::write(temp.join("dx"), r#"
workspace.name="DX"
workspace.root="."
paths.cli="cli"
"#).expect("write root dx");
        fs::write(project.join("dx"), r#"
project(name=demo version=0.1.0 kind=www-app)
www(app_router=true)
"#).expect("write project dx");

        let discovered = discover_config_path(&project).expect("discover root dx");
        assert_eq!(discovered, temp.join("dx"));
    }

    #[test]
    fn skips_serializer_table_config() {
        let temp = tempfile_dir("dx-config-skip-table");
        let project = temp.join("check");
        fs::create_dir_all(&project).expect("create project");
        fs::write(temp.join("dx"), r#"
workspace.name="DX"
workspace.root="."
paths.cli="cli"
"#).expect("write root dx");
        fs::write(project.join("dx"), r#"
name="dx-check-engine"
kind="rust-library"
targets[id role](
c supported
cpp main
)
"#).expect("write project table dx");

        let discovered = discover_config_path(&project).expect("discover root dx");
        assert_eq!(discovered, temp.join("dx"));
    }

    #[test]
    fn default_config_when_no_dx_found() {
        let temp = tempfile_dir("dx-config-no-dx");
        let config = DxConfig::load(temp.as_path()).expect("load default");
        assert_eq!(config.workspace.root, temp);
        assert_eq!(config.paths.cli, temp.join("cli"));
    }

    #[test]
    fn env_override_wins() {
        let dx_home = tempfile_dir("dx-config-env-home");
        let caller = tempfile_dir("dx-config-env-caller");
        fs::write(dx_home.join("dx"), r#"
workspace.name="DX"
workspace.root="."
paths.cli="cli"
"#).expect("write dx config");

        let config = DxConfig::load_with_home(
            &caller,
            Some(dx_home.clone().into_os_string()),
        ).expect("load config");

        assert_eq!(config.workspace.root, dx_home);
        assert_eq!(config.paths.cli, config.workspace.root.join("cli"));
    }

    #[test]
    fn env_override_rejects_relative() {
        let caller = tempfile_dir("dx-config-relative-env");
        let err = DxConfig::load_with_home(
            &caller,
            Some(OsString::from("relative-dx-home")),
        ).expect_err("should fail");

        assert!(err.to_string().contains("must be an absolute path"));
    }

    #[test]
    fn py_path_aliases() {
        let temp = tempfile_dir("dx-config-py-alias");
        let config_path = temp.join("dx");
        fs::write(&config_path, r#"
workspace.name="DX"
workspace.root="."
paths.py="py-runtime"
paths.py_package_manager="py-runtime/package-manager"
"#).expect("write dx config");

        let config = DxConfig::from_path(&config_path).expect("load dx config");
        assert_eq!(config.paths.python, temp.join("py-runtime"));
        assert_eq!(
            config.paths.python_package_manager,
            temp.join("py-runtime").join("package-manager")
        );
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
