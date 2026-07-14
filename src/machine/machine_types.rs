//! RKYV-compatible types for machine format serialization
//!
//! Uses a flattened arena-based approach to avoid recursive types.
//! All nested values are stored in a flat Vec with indices.

use rkyv::{Archive, Deserialize, Serialize};

/// RKYV-compatible document with flattened value arena
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(derive(Debug))]
pub struct MachineDocument {
    /// Top-level context entries mapped to value-arena indexes.
    pub context: Vec<(String, usize)>,
    /// Named reference values preserved from the source document.
    pub refs: Vec<(String, String)>,
    /// Tabular sections keyed by compact section id.
    pub sections: Vec<(char, MachineSection)>,
    /// Human-readable section names keyed by compact section id.
    pub section_names: Vec<(char, String)>,
    /// Original document entry ordering for context and section records.
    pub entry_order: Vec<MachineEntryRef>,
    /// Flat storage for every nested value referenced by indexes.
    pub value_arena: Vec<MachineValue>,
}

/// Reference to a top-level document entry in its original order.
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[rkyv(derive(Debug))]
pub enum MachineEntryRef {
    /// Context entry identified by key.
    Context(String),
    /// Section entry identified by compact section id.
    Section(char),
}

/// RKYV-compatible table section that stores cells as value-arena indexes.
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[rkyv(derive(Debug))]
pub struct MachineSection {
    /// Column names for each row.
    pub schema: Vec<String>,
    /// Rows of value-arena indexes, one index per cell.
    pub rows: Vec<Vec<usize>>,
}

/// Non-recursive value type - arrays/objects store indices
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[rkyv(derive(Debug))]
pub enum MachineValue {
    /// String value stored inline in the arena.
    Str(String),
    /// Floating-point numeric value.
    Num(f64),
    /// Boolean value.
    Bool(bool),
    /// Null value.
    Null,
    /// Array represented by value-arena indexes.
    Arr(Vec<usize>),
    /// Object fields mapped to value-arena indexes.
    Obj(Vec<(String, usize)>),
    /// Reference name that can be resolved by higher-level consumers.
    Ref(String),
}

impl From<&crate::llm::types::DxDocument> for MachineDocument {
    fn from(doc: &crate::llm::types::DxDocument) -> Self {
        use crate::llm::types::EntryRef;

        let mut value_arena = Vec::new();
        let mut context = Vec::new();

        // Convert context values
        for (k, v) in &doc.context {
            let idx = add_value_to_arena(v, &mut value_arena);
            context.push((k.clone(), idx));
        }

        // Convert sections
        let sections: Vec<(char, MachineSection)> = doc
            .sections
            .iter()
            .map(|(k, v)| {
                let schema = v.schema.clone();
                let rows: Vec<Vec<usize>> = v
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|cell| add_value_to_arena(cell, &mut value_arena))
                            .collect()
                    })
                    .collect();
                (*k, MachineSection { schema, rows })
            })
            .collect();

        Self {
            context,
            refs: doc
                .refs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            sections,
            section_names: doc
                .section_names
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            entry_order: doc
                .entry_order
                .iter()
                .map(|e| match e {
                    EntryRef::Context(s) => MachineEntryRef::Context(s.clone()),
                    EntryRef::Section(c) => MachineEntryRef::Section(*c),
                })
                .collect(),
            value_arena,
        }
    }
}

fn add_value_to_arena(v: &crate::llm::types::DxLlmValue, arena: &mut Vec<MachineValue>) -> usize {
    use crate::llm::types::DxLlmValue;

    let idx = arena.len();
    arena.push(MachineValue::Null);
    let machine_value = match v {
        DxLlmValue::Str(s) => MachineValue::Str(s.clone()),
        DxLlmValue::Int(i) => MachineValue::Num(*i as f64),
        DxLlmValue::Num(n) => MachineValue::Num(*n),
        DxLlmValue::Bool(b) => MachineValue::Bool(*b),
        DxLlmValue::Null => MachineValue::Null,
        DxLlmValue::Arr(items) => {
            let indices: Vec<usize> = items
                .iter()
                .map(|item| add_value_to_arena(item, arena))
                .collect();
            MachineValue::Arr(indices)
        }
        DxLlmValue::Obj(fields) => {
            let pairs: Vec<(String, usize)> = fields
                .iter()
                .map(|(k, v)| (k.clone(), add_value_to_arena(v, arena)))
                .collect();
            MachineValue::Obj(pairs)
        }
        DxLlmValue::Ref(r) => MachineValue::Ref(r.clone()),
    };
    arena[idx] = machine_value;
    idx
}

impl From<&MachineDocument> for crate::llm::types::DxDocument {
    fn from(m: &MachineDocument) -> Self {
        use crate::llm::types::{DxSection, EntryRef};

        let mut doc = Self::new();

        // Convert context
        for (k, idx) in &m.context {
            let value = get_value_from_arena(*idx, &m.value_arena);
            doc.context.insert(k.clone(), value);
        }

        doc.refs = m.refs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Convert sections
        for (k, section) in &m.sections {
            let mut dx_section = DxSection::new(section.schema.clone());
            for row_indices in &section.rows {
                let row: Vec<_> = row_indices
                    .iter()
                    .map(|idx| get_value_from_arena(*idx, &m.value_arena))
                    .collect();
                dx_section.rows.push(row);
            }
            doc.sections.insert(*k, dx_section);
        }

        doc.section_names = m
            .section_names
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        doc.entry_order = m
            .entry_order
            .iter()
            .map(|e| match e {
                MachineEntryRef::Context(s) => EntryRef::Context(s.clone()),
                MachineEntryRef::Section(c) => EntryRef::Section(*c),
            })
            .collect();
        doc
    }
}

fn get_value_from_arena(idx: usize, arena: &[MachineValue]) -> crate::llm::types::DxLlmValue {
    use crate::llm::types::DxLlmValue;
    use indexmap::IndexMap;

    match &arena[idx] {
        MachineValue::Str(s) => DxLlmValue::Str(s.clone()),
        MachineValue::Num(n) => DxLlmValue::Num(*n),
        MachineValue::Bool(b) => DxLlmValue::Bool(*b),
        MachineValue::Null => DxLlmValue::Null,
        MachineValue::Arr(indices) => {
            let items: Vec<DxLlmValue> = indices
                .iter()
                .map(|i| get_value_from_arena(*i, arena))
                .collect();
            DxLlmValue::Arr(items)
        }
        MachineValue::Obj(pairs) => {
            let mut fields = IndexMap::new();
            for (k, i) in pairs {
                fields.insert(k.clone(), get_value_from_arena(*i, arena));
            }
            DxLlmValue::Obj(fields)
        }
        MachineValue::Ref(r) => DxLlmValue::Ref(r.clone()),
    }
}
