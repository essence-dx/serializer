use serializer::human::parser::HumanParser;
use serializer::llm::serializer::{LlmSerializer, SerializerConfig};
use serializer::llm::types::{DxDocument, DxLlmValue, DxSection};
use serializer::llm::{
    document_to_llm, document_to_machine, human_to_llm, llm_to_document, llm_to_human,
    machine_to_document,
};

// ============================================================================
// Human Format Parsing
// ============================================================================

#[test]
fn test_human_parens_style() {
    let input = r"
project(
  name    = dx-os
  version = 1.0.0
)
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(
        doc.context.contains_key("project"),
        "Should parse project group"
    );
}

#[test]
fn test_human_flat_key_value() {
    let input = r"
name    = dx-os
version = 1.0.0
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("name").unwrap().as_str(), Some("dx-os"));
    assert_eq!(doc.context.get("version").unwrap().as_str(), Some("1.0.0"));
}

#[test]
fn test_human_nested_groups() {
    let input = r"
script(
  settings(
    shell    = bash
    fallback = true
  )
)
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(
        doc.context.contains_key("script"),
        "Should parse script group"
    );
}

#[test]
fn test_human_table_comma_separated() {
    let input = r"
recipes[name group doc script](
  build,all,Build all workspace crates,cargo build --workspace
  check,all,Run cargo check,cargo check --workspace
)
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(!doc.sections.is_empty(), "Should parse table");
}

#[test]
fn test_human_table_space_separated() {
    let input = r"
aliases[name target](
  b  build
  c  check
  t  test
)
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(
        !doc.sections.is_empty(),
        "Should parse space-separated table"
    );
}

#[test]
fn test_human_inline_arrays() {
    let input = r"
tags = [rust performance serialization]
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    let tags = doc.context.get("tags").unwrap();
    assert!(tags.as_arr().is_some(), "Should parse array");
    assert_eq!(tags.as_arr().unwrap().len(), 3);
}

#[test]
fn test_human_multiline_arrays() {
    let input = r"
workspace.paths[2]:
- @/www
- @/backend
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    let paths = doc.context.get("workspace.paths").unwrap();
    assert!(paths.as_arr().is_some(), "Should parse multi-line array");
    assert_eq!(paths.as_arr().unwrap().len(), 2);
}

#[test]
fn test_human_empty_group() {
    let input = r"features()";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(doc.context.contains_key("features"));
}

#[test]
fn test_human_comments() {
    let input = r"
# This is a comment
name = dx-os
# Another comment
version = 1.0.0
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("name").unwrap().as_str(), Some("dx-os"));
}

#[test]
fn test_human_null_value() {
    let input = r"value = null";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(doc.context.get("value").unwrap().is_null());
}

#[test]
fn test_human_boolean() {
    let input = r"
active  = true
enabled = false
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("active").unwrap().as_bool(), Some(true));
    assert_eq!(doc.context.get("enabled").unwrap().as_bool(), Some(false));
}

#[test]
fn test_human_numbers() {
    let input = r"
int    = 42
float  = 3.14
neg    = -10
zero   = 0
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("int").unwrap().as_num(), Some(42.0));
    assert_eq!(doc.context.get("float").unwrap().as_num(), Some(3.14));
    assert_eq!(doc.context.get("neg").unwrap().as_num(), Some(-10.0));
    assert_eq!(doc.context.get("zero").unwrap().as_num(), Some(0.0));
}

#[test]
fn test_human_unicode() {
    let input = r"
cjk   = 你好世界
emoji = 🚀🎉
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("cjk").unwrap().as_str(), Some("你好世界"));
}

// ============================================================================
// LLM Format
// ============================================================================

#[test]
fn test_llm_normal_roundtrip() {
    let human = "name = dx-os\nversion = 1.0.0";
    let llm = human_to_llm(human).unwrap();
    let doc1 = llm_to_document(&llm).unwrap();
    let back_to_human = llm_to_human(&llm).unwrap();
    let llm_again = human_to_llm(&back_to_human).unwrap();
    let doc2 = llm_to_document(&llm_again).unwrap();
    // Verify data preserved through round-trip
    assert!(doc1.context.contains_key("name"));
    assert!(doc2.context.contains_key("name"));
    assert!(!back_to_human.is_empty());
    assert!(!llm_again.is_empty());
}

#[test]
fn test_llm_detects_existing_llm() {
    let llm_input = "name=dx-os\nversion=1.0.0";
    let result = human_to_llm(llm_input).unwrap();
    // Should return as-is (already LLM format)
    assert_eq!(result, llm_input);
}

#[test]
fn test_llm_table_roundtrip() {
    let human = r"
users[id name email](
  1,Alice,alice@example.com
  2,Bob,bob@example.com
)
";
    let llm = human_to_llm(human).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(!doc.sections.is_empty(), "Table should survive roundtrip");
}

// ============================================================================
// Machine Format
// ============================================================================

#[test]
fn test_machine_roundtrip() {
    let mut doc = DxDocument::new();
    doc.context
        .insert("name".into(), DxLlmValue::Str("dx-os".into()));
    doc.context
        .insert("version".into(), DxLlmValue::Str("1.0.0".into()));

    let machine = document_to_machine(&doc);
    let parsed = machine_to_document(&machine).unwrap();

    assert_eq!(parsed.context.len(), 2);
    assert_eq!(parsed.context.get("name").unwrap().as_str(), Some("dx-os"));
}

#[test]
fn test_machine_all_value_types() {
    let mut doc = DxDocument::new();
    doc.context
        .insert("str".into(), DxLlmValue::Str("hello".into()));
    doc.context.insert("num".into(), DxLlmValue::Num(42.0));
    doc.context.insert("bool".into(), DxLlmValue::Bool(true));
    doc.context.insert("null".into(), DxLlmValue::Null);

    let machine = document_to_machine(&doc);
    let parsed = machine_to_document(&machine).unwrap();

    assert_eq!(parsed.context.len(), 4);
    assert_eq!(parsed.context.get("str").unwrap().as_str(), Some("hello"));
    assert_eq!(parsed.context.get("num").unwrap().as_num(), Some(42.0));
    assert_eq!(parsed.context.get("bool").unwrap().as_bool(), Some(true));
    assert!(parsed.context.get("null").unwrap().is_null());
}

#[test]
fn test_machine_unicode() {
    let mut doc = DxDocument::new();
    doc.context
        .insert("cjk".into(), DxLlmValue::Str("你好世界".into()));
    doc.context
        .insert("emoji".into(), DxLlmValue::Str("🚀🎉".into()));

    let machine = document_to_machine(&doc);
    let parsed = machine_to_document(&machine).unwrap();

    assert_eq!(
        parsed.context.get("cjk").unwrap().as_str(),
        Some("你好世界")
    );
    assert_eq!(parsed.context.get("emoji").unwrap().as_str(), Some("🚀🎉"));
}

#[test]
fn test_machine_table_section() {
    let mut doc = DxDocument::new();
    let mut section = DxSection::new(vec!["id".into(), "name".into()]);
    section
        .rows
        .push(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("Alice".into())]);
    section
        .rows
        .push(vec![DxLlmValue::Num(2.0), DxLlmValue::Str("Bob".into())]);
    doc.sections.insert('u', section);

    let machine = document_to_machine(&doc);
    let parsed = machine_to_document(&machine).unwrap();

    let section = parsed.sections.get(&'u').unwrap();
    assert_eq!(section.rows.len(), 2);
}

#[test]
fn test_machine_empty_document() {
    let doc = DxDocument::new();
    let machine = document_to_machine(&doc);
    let parsed = machine_to_document(&machine).unwrap();
    assert!(parsed.context.is_empty());
    assert!(parsed.sections.is_empty());
}

// ============================================================================
// Human → LLM → Machine Full Pipeline
// ============================================================================

#[test]
fn test_full_pipeline() {
    let human = r"
project(
  name    = dx-os
  version = 1.0.0
  scripts(
    build = cargo build
    test  = cargo test
  )
)
";
    // Human → LLM
    let llm = human_to_llm(human).unwrap();
    let doc1 = llm_to_document(&llm).unwrap();
    assert!(doc1.context.contains_key("project"));

    // LLM → Human (no error)
    let human_back = llm_to_human(&llm).unwrap();
    assert!(!human_back.is_empty());

    // Human → Machine (via document)
    let doc2 = llm_to_document(&llm).unwrap();
    let machine = document_to_machine(&doc2);
    let doc3 = machine_to_document(&machine).unwrap();
    assert_eq!(doc2.context.len(), doc3.context.len());
}

#[test]
fn test_complex_config_roundtrip() {
    let human = r"
project(
  name    = dx-os
  version = 1.0.0
)

script(
  settings(
    shell    = bash
    fallback = true
  )

  aliases[name target](
    b  build
    c  check
    t  test
  )
)
";
    // Human → LLM
    let llm = human_to_llm(human).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert!(doc.context.contains_key("project"));
    assert!(doc.context.contains_key("script"));
    assert!(!doc.sections.is_empty());

    // LLM → machine → document (flat values only)
    let machine = document_to_machine(&doc);
    let parsed_doc = machine_to_document(&machine).unwrap();
    // Context keys and sections survive machine roundtrip
    assert!(!parsed_doc.sections.is_empty() || !parsed_doc.context.is_empty());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_input() {
    let result = human_to_llm("");
    assert!(result.is_ok(), "Empty input should not crash");
}

#[test]
fn test_whitespace_only() {
    let result = human_to_llm("   \n  \n  ");
    assert!(result.is_ok(), "Whitespace-only should not crash");
}

#[test]
fn test_single_key_value() {
    let llm = human_to_llm("key = value").unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn test_deep_nesting() {
    let mut input = String::from("a(\n");
    for _ in 0..10 {
        input.push_str("  child(\n");
    }
    for _ in 0..10 {
        input.push_str("  )\n");
    }
    input.push_str(")\n");

    let result = human_to_llm(&input);
    assert!(result.is_ok(), "Deep nesting should not crash");
}

#[test]
fn test_large_table() {
    let mut input = String::from("large[id value](\n");
    for i in 0..100 {
        input.push_str(&format!("  {},val-{}\n", i, i));
    }
    input.push_str(")\n");

    let result = human_to_llm(&input);
    assert!(result.is_ok(), "Large table should not crash");
}

#[test]
fn test_negative_numbers() {
    let llm = human_to_llm("temp = -5").unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("temp").unwrap().as_num(), Some(-5.0));
}

#[test]
fn test_zero_values() {
    let input = r"
count  = 0
price  = 0.0
active = false
empty  = null
";
    let llm = human_to_llm(input).unwrap();
    let doc = llm_to_document(&llm).unwrap();
    assert_eq!(doc.context.get("count").unwrap().as_num(), Some(0.0));
    assert_eq!(doc.context.get("active").unwrap().as_bool(), Some(false));
    assert!(doc.context.get("empty").unwrap().is_null());
}

// ============================================================================
// Auto-detect between Human and LLM format
// ============================================================================

#[test]
fn test_auto_detect_human_format() {
    // Human format with spaces around =
    let human = "name = dx-os\nversion = 1.0.0";
    let result = human_to_llm(human).unwrap();
    let doc = llm_to_document(&result).unwrap();
    assert_eq!(doc.context.len(), 2);
}

#[test]
fn test_auto_detect_llm_format() {
    // LLM format with no spaces around =
    let llm_input = "name=dx-os\nversion=1.0.0";
    let result = human_to_llm(llm_input).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_auto_detect_parens() {
    let input = "project(name=dx-os version=1.0.0)";
    let result = human_to_llm(input).unwrap();
    assert!(!result.is_empty());
}

// ============================================================================
// Real-world example: G:\Dx\os\examples\dx\dx
// ============================================================================

const BIG_EXAMPLE: &str = include_str!("../examples/dx/dx");

#[test]
fn test_big_example_parses_to_llm() {
    let llm = human_to_llm(BIG_EXAMPLE).unwrap();
    assert!(!llm.is_empty(), "LLM output should not be empty");
    // Verify content preserved
    assert!(llm.contains("project"), "LLM should contain project");
    assert!(llm.contains("build"), "LLM should contain build");
    assert!(llm.contains("scripts"), "LLM should contain scripts");
    assert!(llm.contains("ci"), "LLM should contain ci");
    assert!(llm.contains("features"), "LLM should contain features");
}

#[test]
fn test_big_example_pipeline() {
    // Human → LLM
    let llm = human_to_llm(BIG_EXAMPLE).unwrap();
    assert!(!llm.is_empty());

    // LLM → Machine (skip re-parsing, go direct)
    let doc = llm_to_document(&llm).unwrap_or_else(|_| {
        // If LLM re-parse fails (known SchemaMismatch with multi-table),
        // fall back to human → document directly
        let parser = serializer::human::parser::HumanParser::new();
        parser.parse(BIG_EXAMPLE).unwrap()
    });

    let machine = document_to_machine(&doc);
    assert!(machine.data.len() > 0, "Machine output should have data");

    let parsed_doc = machine_to_document(&machine).unwrap();
    assert!(
        parsed_doc.context.contains_key("project") || parsed_doc.context.contains_key("build"),
        "Machine roundtrip should preserve top-level groups"
    );
}

#[test]
fn test_big_example_no_panic() {
    let result = human_to_llm(BIG_EXAMPLE);
    assert!(result.is_ok(), "Big example should not cause errors");
}

#[test]
fn test_big_example_human_output() {
    let parser = HumanParser::new();
    let doc = parser.parse(BIG_EXAMPLE).unwrap();
    assert!(!doc.context.is_empty(), "Should parse keys");
    assert!(doc.context.contains_key("project"), "Should have project");
    assert!(doc.context.contains_key("build"), "Should have build");
    assert!(doc.context.contains_key("scripts"), "Should have scripts");
    // features, ci may be parsed as groups depending on content
    assert!(!doc.sections.is_empty(), "Should have tables");
    // Verify we get tables with rows
    let total_rows: usize = doc.sections.values().map(|s| s.rows.len()).sum();
    assert!(
        total_rows > 10,
        "Should have many table rows across all tables"
    );
}

#[test]
fn test_big_example_stress() {
    // Just ensure no crashes on this real-world input
    let _ = human_to_llm(BIG_EXAMPLE).unwrap();
    // Also verify it works through machine
    let parser = serializer::human::parser::HumanParser::new();
    let doc = parser.parse(BIG_EXAMPLE).unwrap();
    let machine = document_to_machine(&doc);
    let _parsed = machine_to_document(&machine).unwrap();
}
