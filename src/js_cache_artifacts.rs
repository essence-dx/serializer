use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct CatalogJson {
    schema: String,
    #[serde(rename = "generatedAtUtc")]
    generated_at_utc: String,
    shards: Vec<String>,
    entries: Vec<CatalogJsonEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct CatalogJsonEntry {
    key: String,
    kind: String,
    source: String,
    shard: String,
    machine: String,
    metadata: String,
    #[serde(rename = "keyInterning")]
    key_interning: Option<String>,
    #[serde(rename = "sourceBytes")]
    source_bytes: u64,
    #[serde(rename = "sourceModifiedUnixMs")]
    source_modified_unix_ms: Option<u64>,
    #[serde(rename = "sourceBlake3")]
    source_blake3: String,
    #[serde(rename = "machineBlake3")]
    machine_blake3: String,
    #[serde(rename = "machineBytes")]
    machine_bytes: u64,
    #[serde(rename = "metadataBytes")]
    metadata_bytes: u64,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
struct JsCacheCatalogMachine {
    schema: String,
    generated_at_utc: String,
    shards: Vec<String>,
    lookup: Vec<JsCacheCatalogLookup>,
    entries: Vec<JsCacheCatalogEntryMachine>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
struct JsCacheCatalogLookup {
    key: String,
    index: u32,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
struct JsCacheCatalogEntryMachine {
    key: String,
    kind: String,
    source: String,
    shard: String,
    machine: String,
    metadata: String,
    key_interning: Option<String>,
    source_bytes: u64,
    source_modified_unix_ms: Option<u64>,
    source_blake3: String,
    machine_blake3: String,
    machine_bytes: u64,
    metadata_bytes: u64,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
struct JsCacheShardMachine {
    schema: String,
    shard: String,
    entries: Vec<JsCacheShardEntryMachine>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
struct JsCacheShardEntryMachine {
    key: String,
    source: String,
    machine: String,
    metadata: String,
    key_interning: Option<String>,
    source_blake3: String,
    machine_blake3: String,
    machine_document: Option<Vec<u8>>,
    package_json_read: Option<PackageJsonReadMachine>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
struct PackageJsonReadMachine {
    name: Option<String>,
    version: Option<String>,
    module_type: Option<String>,
    main: Option<String>,
    module: Option<String>,
    browser: Option<usize>,
    jsnext_main: Option<String>,
    side_effects: Option<usize>,
    exports: Option<usize>,
    imports: Option<usize>,
    value_arena: Vec<PackageJsonReadMachineValue>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug)]
#[rkyv(derive(Debug))]
enum PackageJsonReadMachineValue {
    Str(String),
    Bool(bool),
    Null,
    Arr(Vec<usize>),
    Obj(Vec<(String, usize)>),
}

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DxJsMachineCachePackedShardHeader {
    magic: [u8; 8],
    version: u32,
    header_bytes: u32,
    kind_id: u32,
    entry_count: u32,
    source_bytes: u64,
    machine_bytes: u64,
    metadata_bytes: u64,
    shard_path_blake3: [u8; 32],
    source_identity_blake3: [u8; 32],
    machine_identity_blake3: [u8; 32],
    reserved: [u8; 16],
}

const CATALOG_JSON_SCHEMA: &str = "dx.js.machine_cache_catalog.v1";
const CATALOG_MACHINE_SCHEMA: &str = "dx.js.machine_cache_catalog.machine.rkyv_hashbrown.v1";
const SHARD_MACHINE_SCHEMA: &str =
    "dx.js.machine_cache_packed_shard.rkyv_package_json_resolver_read_identity.v5";
const SHARD_MAGIC: [u8; 8] = *b"DXJSHARD";
const SHARD_HEADER_BYTES: u32 = 160;

pub fn write_js_cache_artifacts(
    catalog_json_path: &Path,
    output_dir: &Path,
    shard_identity_root: Option<&Path>,
) -> Result<(), String> {
    let text = fs::read_to_string(catalog_json_path)
        .map_err(|error| format!("read {}: {error}", catalog_json_path.display()))?;
    let mut catalog: CatalogJson = serde_json::from_str(&text)
        .map_err(|error| format!("parse {}: {error}", catalog_json_path.display()))?;

    if catalog.schema != CATALOG_JSON_SCHEMA {
        return Err(format!(
            "unexpected DX JS catalog schema {} in {}",
            catalog.schema,
            catalog_json_path.display()
        ));
    }

    catalog
        .entries
        .sort_by(|left, right| left.key.cmp(&right.key));
    let mut lookup = hashbrown::HashMap::with_capacity(catalog.entries.len());
    for (index, entry) in catalog.entries.iter().enumerate() {
        if lookup.insert(entry.key.clone(), index as u32).is_some() {
            return Err(format!("duplicate DX JS catalog key: {}", entry.key));
        }
    }

    let catalog_machine = JsCacheCatalogMachine {
        schema: CATALOG_MACHINE_SCHEMA.to_string(),
        generated_at_utc: catalog.generated_at_utc.clone(),
        shards: catalog.shards.clone(),
        lookup: catalog
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| JsCacheCatalogLookup {
                key: entry.key.clone(),
                index: index as u32,
            })
            .collect(),
        entries: catalog
            .entries
            .iter()
            .map(JsCacheCatalogEntryMachine::from)
            .collect(),
    };

    fs::create_dir_all(output_dir)
        .map_err(|error| format!("create {}: {error}", output_dir.display()))?;
    let catalog_machine_path = output_dir.join("catalog.machine");
    write_atomic(
        &catalog_machine_path,
        serializer::machine::api::serialize(&catalog_machine)
            .map_err(|error| format!("serialize catalog.machine: {error}"))?
            .as_ref(),
    )
    .map_err(|error| format!("write {}: {error}", catalog_machine_path.display()))?;

    let default_shard_identity_root = output_dir.join("shards");
    let shard_identity_root = shard_identity_root.unwrap_or(default_shard_identity_root.as_path());
    write_shards(output_dir, shard_identity_root, &catalog)?;

    Ok(())
}

fn write_shards(
    output_dir: &Path,
    shard_identity_root: &Path,
    catalog: &CatalogJson,
) -> Result<(), String> {
    let mut by_shard: BTreeMap<&str, Vec<&CatalogJsonEntry>> = BTreeMap::new();
    for entry in &catalog.entries {
        by_shard.entry(&entry.shard).or_default().push(entry);
    }

    for (shard, entries) in by_shard {
        if entries.is_empty() {
            continue;
        }

        let kind = &entries[0].kind;
        let shard_relative_path = format!(
            "{}/{}.dxjs",
            normalize_path(shard_identity_root),
            shard.replace('\\', "/")
        );
        let shard_path = output_dir
            .join("shards")
            .join(shard.replace('/', std::path::MAIN_SEPARATOR_STR))
            .with_extension("dxjs");

        if let Some(parent) = shard_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create {}: {error}", parent.display()))?;
        }

        let shard_entries = entries
            .iter()
            .map(|entry| {
                Ok(JsCacheShardEntryMachine {
                    key: entry.key.clone(),
                    source: entry.source.clone(),
                    machine: entry.machine.clone(),
                    metadata: entry.metadata.clone(),
                    key_interning: entry.key_interning.clone(),
                    source_blake3: entry.source_blake3.clone(),
                    machine_blake3: entry.machine_blake3.clone(),
                    machine_document: packed_machine_document_for_entry(entry)?,
                    package_json_read: package_json_read_for_entry(entry)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let source_bytes = entries.iter().map(|entry| entry.source_bytes).sum();
        let machine_bytes = entries.iter().map(|entry| entry.machine_bytes).sum();
        let metadata_bytes = entries.iter().map(|entry| entry.metadata_bytes).sum();
        let identity_input = shard_identity_input(&shard_entries);
        let machine_identity_input = format!("machine:{identity_input}");
        let header = DxJsMachineCachePackedShardHeader {
            magic: SHARD_MAGIC,
            version: 5,
            header_bytes: SHARD_HEADER_BYTES,
            kind_id: packed_shard_kind_id(kind),
            entry_count: entries.len() as u32,
            source_bytes,
            machine_bytes,
            metadata_bytes,
            shard_path_blake3: blake3::hash(shard_relative_path.as_bytes()).into(),
            source_identity_blake3: blake3::hash(identity_input.as_bytes()).into(),
            machine_identity_blake3: blake3::hash(machine_identity_input.as_bytes()).into(),
            reserved: [0; 16],
        };

        let shard_machine = JsCacheShardMachine {
            schema: SHARD_MACHINE_SCHEMA.to_string(),
            shard: shard.to_string(),
            entries: shard_entries,
        };
        let body = serializer::machine::api::serialize(&shard_machine)
            .map_err(|error| format!("serialize shard {shard}: {error}"))?;

        let mut bytes = Vec::with_capacity(SHARD_HEADER_BYTES as usize + body.len());
        bytes.extend_from_slice(bytemuck::bytes_of(&header));
        bytes.extend_from_slice(body.as_ref());
        write_atomic(&shard_path, &bytes)
            .map_err(|error| format!("write {}: {error}", shard_path.display()))?;
    }

    Ok(())
}

impl From<&CatalogJsonEntry> for JsCacheCatalogEntryMachine {
    fn from(entry: &CatalogJsonEntry) -> Self {
        Self {
            key: entry.key.clone(),
            kind: entry.kind.clone(),
            source: entry.source.clone(),
            shard: entry.shard.clone(),
            machine: entry.machine.clone(),
            metadata: entry.metadata.clone(),
            key_interning: entry.key_interning.clone(),
            source_bytes: entry.source_bytes,
            source_modified_unix_ms: entry.source_modified_unix_ms,
            source_blake3: entry.source_blake3.clone(),
            machine_blake3: entry.machine_blake3.clone(),
            machine_bytes: entry.machine_bytes,
            metadata_bytes: entry.metadata_bytes,
        }
    }
}

fn shard_identity_input(entries: &[JsCacheShardEntryMachine]) -> String {
    let mut input = String::new();
    for entry in entries {
        input.push_str(&entry.key);
        input.push('\0');
        input.push_str(&entry.source_blake3);
        input.push('\0');
        input.push_str(&entry.machine_blake3);
        input.push('\0');
        match &entry.package_json_read {
            Some(read) => input.push_str(package_json_read_identity(read).to_hex().as_str()),
            None => input.push_str("none"),
        }
        input.push('\0');
    }
    input
}

fn package_json_read_identity(read: &PackageJsonReadMachine) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"dx-package-json-read-v2");
    update_optional_string_hash(&mut hasher, read.name.as_deref());
    update_optional_string_hash(&mut hasher, read.version.as_deref());
    update_optional_string_hash(&mut hasher, read.module_type.as_deref());
    update_optional_string_hash(&mut hasher, read.main.as_deref());
    update_optional_string_hash(&mut hasher, read.module.as_deref());
    update_optional_usize_hash(&mut hasher, read.browser);
    update_optional_string_hash(&mut hasher, read.jsnext_main.as_deref());
    update_optional_usize_hash(&mut hasher, read.side_effects);
    update_optional_usize_hash(&mut hasher, read.exports);
    update_optional_usize_hash(&mut hasher, read.imports);
    update_u64_hash(&mut hasher, read.value_arena.len() as u64);
    for value in &read.value_arena {
        update_package_json_read_value_hash(&mut hasher, value);
    }
    hasher.finalize()
}

fn update_optional_string_hash(hasher: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            update_str_hash(hasher, value);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_optional_usize_hash(hasher: &mut blake3::Hasher, value: Option<usize>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            update_u64_hash(hasher, value as u64);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_package_json_read_value_hash(
    hasher: &mut blake3::Hasher,
    value: &PackageJsonReadMachineValue,
) {
    match value {
        PackageJsonReadMachineValue::Str(value) => {
            hasher.update(b"s");
            update_str_hash(hasher, value);
        }
        PackageJsonReadMachineValue::Bool(value) => {
            hasher.update(&[b'b', u8::from(*value)]);
        }
        PackageJsonReadMachineValue::Null => {
            hasher.update(b"n");
        }
        PackageJsonReadMachineValue::Arr(items) => {
            hasher.update(b"a");
            update_u64_hash(hasher, items.len() as u64);
            for index in items {
                update_u64_hash(hasher, *index as u64);
            }
        }
        PackageJsonReadMachineValue::Obj(fields) => {
            hasher.update(b"o");
            update_u64_hash(hasher, fields.len() as u64);
            for (key, index) in fields {
                update_str_hash(hasher, key);
                update_u64_hash(hasher, *index as u64);
            }
        }
    }
}

fn update_str_hash(hasher: &mut blake3::Hasher, value: &str) {
    update_u64_hash(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn update_u64_hash(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

fn packed_shard_kind_id(kind: &str) -> u32 {
    match kind {
        "package_json" => 1,
        "tsconfig" => 2,
        "bunfig" => 3,
        _ => {
            let mut hash = 2_166_136_261u32;
            for byte in kind.bytes() {
                hash ^= u32::from(byte);
                hash = hash.wrapping_mul(16_777_619);
            }
            1024 + (hash % (u32::MAX - 1024))
        }
    }
}

fn packed_machine_document_for_entry(entry: &CatalogJsonEntry) -> Result<Option<Vec<u8>>, String> {
    if entry.kind != "package_json" {
        return Ok(None);
    }

    let bytes = fs::read(&entry.machine)
        .map_err(|error| format!("read packed machine document {}: {error}", entry.machine))?;
    if bytes.len() as u64 != entry.machine_bytes {
        return Err(format!(
            "packed machine document byte length mismatch for {}: expected {} actual {}",
            entry.machine,
            entry.machine_bytes,
            bytes.len()
        ));
    }

    let actual_blake3 = blake3::hash(&bytes).to_hex().to_string();
    if actual_blake3 != entry.machine_blake3 {
        return Err(format!(
            "packed machine document blake3 mismatch for {}: expected {} actual {}",
            entry.machine, entry.machine_blake3, actual_blake3
        ));
    }

    Ok(Some(bytes))
}

fn package_json_read_for_entry(
    entry: &CatalogJsonEntry,
) -> Result<Option<PackageJsonReadMachine>, String> {
    if entry.kind != "package_json" {
        return Ok(None);
    }

    let source = fs::read_to_string(&entry.source)
        .map_err(|error| format!("read package-json source {}: {error}", entry.source))?;
    let document = serializer::json_to_document(&source)
        .map_err(|error| format!("parse package-json source {}: {error}", entry.source))?;

    let mut value_arena = Vec::new();
    let browser = match document.context.get("browser") {
        Some(value) => match add_package_json_browser_value(value, &mut value_arena) {
            Some(index) => index,
            None => return Ok(None),
        },
        None => None,
    };
    let side_effects = document
        .context
        .get("sideEffects")
        .and_then(|value| add_package_json_side_effects_value(value, &mut value_arena));
    Ok(Some(PackageJsonReadMachine {
        name: package_json_read_string(&document, "name"),
        version: package_json_read_string(&document, "version"),
        module_type: package_json_read_string(&document, "type"),
        main: package_json_read_string(&document, "main"),
        module: package_json_read_string(&document, "module"),
        browser,
        jsnext_main: package_json_read_string(&document, "jsnext:main"),
        side_effects,
        exports: document
            .context
            .get("exports")
            .and_then(|value| add_package_json_read_value(value, &mut value_arena)),
        imports: document
            .context
            .get("imports")
            .and_then(|value| add_package_json_read_value(value, &mut value_arena)),
        value_arena,
    }))
}

fn package_json_read_string(document: &serializer::DxDocument, key: &str) -> Option<String> {
    document
        .context
        .get(key)
        .and_then(serializer::DxLlmValue::as_str)
        .map(str::to_owned)
}

fn add_package_json_browser_value(
    value: &serializer::DxLlmValue,
    arena: &mut Vec<PackageJsonReadMachineValue>,
) -> Option<Option<usize>> {
    let serializer::DxLlmValue::Obj(fields) = value else {
        return Some(None);
    };

    let index = arena.len();
    arena.push(PackageJsonReadMachineValue::Null);
    let mut indexes = Vec::with_capacity(fields.len());
    for (key, value) in fields {
        match value {
            serializer::DxLlmValue::Str(_) | serializer::DxLlmValue::Bool(_) => {
                let Some(value_index) = add_package_json_read_value(value, arena) else {
                    arena.truncate(index);
                    return None;
                };
                indexes.push((key.clone(), value_index));
            }
            _ => {
                arena.truncate(index);
                return None;
            }
        }
    }
    arena[index] = PackageJsonReadMachineValue::Obj(indexes);
    Some(Some(index))
}

fn add_package_json_side_effects_value(
    value: &serializer::DxLlmValue,
    arena: &mut Vec<PackageJsonReadMachineValue>,
) -> Option<usize> {
    match value {
        serializer::DxLlmValue::Bool(false) => add_package_json_read_value(value, arena),
        serializer::DxLlmValue::Bool(true) => None,
        serializer::DxLlmValue::Arr(items) => {
            let index = arena.len();
            arena.push(PackageJsonReadMachineValue::Null);
            let mut indexes = Vec::with_capacity(items.len());
            for item in items {
                let serializer::DxLlmValue::Str(_) = item else {
                    continue;
                };
                let Some(item_index) = add_package_json_read_value(item, arena) else {
                    arena.truncate(index);
                    return None;
                };
                indexes.push(item_index);
            }
            arena[index] = PackageJsonReadMachineValue::Arr(indexes);
            Some(index)
        }
        _ => None,
    }
}

fn add_package_json_read_value(
    value: &serializer::DxLlmValue,
    arena: &mut Vec<PackageJsonReadMachineValue>,
) -> Option<usize> {
    let index = arena.len();
    arena.push(PackageJsonReadMachineValue::Null);
    let machine_value = match value {
        serializer::DxLlmValue::Str(value) => PackageJsonReadMachineValue::Str(value.clone()),
        serializer::DxLlmValue::Bool(value) => PackageJsonReadMachineValue::Bool(*value),
        serializer::DxLlmValue::Null => PackageJsonReadMachineValue::Null,
        serializer::DxLlmValue::Arr(items) => {
            let mut indexes = Vec::with_capacity(items.len());
            for item in items {
                let Some(item_index) = add_package_json_read_value(item, arena) else {
                    arena.truncate(index);
                    return None;
                };
                indexes.push(item_index);
            }
            PackageJsonReadMachineValue::Arr(indexes)
        }
        serializer::DxLlmValue::Obj(fields) => {
            let mut indexes = Vec::with_capacity(fields.len());
            for (key, value) in fields {
                let Some(value_index) = add_package_json_read_value(value, arena) else {
                    arena.truncate(index);
                    return None;
                };
                indexes.push((key.clone(), value_index));
            }
            PackageJsonReadMachineValue::Obj(indexes)
        }
        serializer::DxLlmValue::Num(_) | serializer::DxLlmValue::Ref(_) => {
            arena.truncate(index);
            return None;
        }
    };
    arena[index] = machine_value;
    Some(index)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp_path = temp_path(path);
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("tmp");
    temp.set_extension(format!("{extension}.tmp"));
    temp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_rkyv_catalog_and_bytemuck_shard_header() {
        let root = std::env::current_dir()
            .unwrap()
            .join(".tmp")
            .join(format!("js-cache-artifacts-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();

        let catalog_json = root.join("catalog.json");
        let machine_path = root.join(".dx/js/package-json.machine");
        fs::create_dir_all(machine_path.parent().unwrap()).unwrap();
        let machine_bytes = b"01234567890123456789".to_vec();
        fs::write(&machine_path, &machine_bytes).unwrap();
        let current_dir = std::env::current_dir().unwrap();
        let machine_relative = normalize_path(
            machine_path
                .strip_prefix(&current_dir)
                .unwrap_or(machine_path.as_path()),
        );
        let machine_blake3 = blake3::hash(&machine_bytes).to_hex().to_string();
        let machine_bytes_len = machine_bytes.len();
        let source_path = root.join("package.json");
        let source_text = r#"{"name":"pkg","version":"1.0.0","type":"module","main":"./index.cjs","module":"./index.mjs","browser":{"./index.cjs":"./browser.cjs"},"sideEffects":false,"exports":{".":"./index.ts"}}"#;
        fs::write(&source_path, source_text).unwrap();
        let source_path_text = normalize_path(&source_path);
        let source_bytes_len = source_text.len();
        fs::write(
            &catalog_json,
            format!(
                r#"{{
  "schema": "dx.js.machine_cache_catalog.v1",
  "generatedAtUtc": "2026-05-29T00:00:00.000Z",
  "shards": ["package_json/aa/698fa04970d1c029"],
  "entries": [{{
    "key": "package_json\u0000package.json",
    "kind": "package_json",
    "source": "{source_path_text}",
    "shard": "package_json/aa/698fa04970d1c029",
    "machine": "{machine_relative}",
    "metadata": ".dx/js/package-json.machine.meta.json",
    "keyInterning": ".dx/js/package-json.keys.json",
    "sourceBytes": {source_bytes_len},
    "sourceModifiedUnixMs": 1700000000000,
    "sourceBlake3": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "machineBlake3": "{machine_blake3}",
    "machineBytes": {machine_bytes_len},
    "metadataBytes": 30
  }}]
}}"#,
            ),
        )
        .unwrap();

        write_js_cache_artifacts(&catalog_json, &root, Some(Path::new(".dx/js/shards"))).unwrap();

        let catalog_machine = fs::read(root.join("catalog.machine")).unwrap();
        assert!(!catalog_machine.starts_with(b"DXJSCAT1"));
        assert!(catalog_machine.len() > 32);

        let shard = fs::read(root.join("shards/package_json/aa/698fa04970d1c029.dxjs")).unwrap();
        assert_eq!(&shard[0..8], b"DXJSHARD");
        assert_eq!(read_u32(&shard, 8), 5);
        assert_eq!(read_u32(&shard, 12), SHARD_HEADER_BYTES);
        assert_eq!(read_u32(&shard, 16), 1);
        assert_eq!(read_u32(&shard, 20), 1);
        assert_eq!(read_u64(&shard, 24), source_bytes_len as u64);
        assert_eq!(read_u64(&shard, 32), 20);
        assert_eq!(read_u64(&shard, 40), 30);
        assert!(shard.len() > SHARD_HEADER_BYTES as usize);
        let body = &shard[SHARD_HEADER_BYTES as usize..];
        let archived =
            rkyv::access::<ArchivedJsCacheShardMachine, rkyv::rancor::Error>(body).unwrap();
        assert_eq!(archived.schema.as_str(), SHARD_MACHINE_SCHEMA);
        let entry = archived.entries.first().unwrap();
        let document = entry.machine_document.as_ref().unwrap();
        assert_eq!(document.as_slice(), machine_bytes.as_slice());
        let package_json_read = entry.package_json_read.as_ref().unwrap();
        assert_eq!(package_json_read.name.as_ref().unwrap().as_str(), "pkg");
        assert_eq!(
            package_json_read.version.as_ref().unwrap().as_str(),
            "1.0.0"
        );
        assert_eq!(
            package_json_read.module_type.as_ref().unwrap().as_str(),
            "module"
        );
        assert_eq!(
            package_json_read.main.as_ref().unwrap().as_str(),
            "./index.cjs"
        );
        assert_eq!(
            package_json_read.module.as_ref().unwrap().as_str(),
            "./index.mjs"
        );
        assert!(package_json_read.browser.is_some());
        assert!(package_json_read.side_effects.is_some());
        assert!(package_json_read.exports.is_some());

        fs::remove_dir_all(root).unwrap();
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn read_u64(bytes: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
    }
}
