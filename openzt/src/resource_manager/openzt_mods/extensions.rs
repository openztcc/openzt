//! Extension mod storage and retrieval system.

use std::{collections::HashMap, sync::Mutex};
use std::sync::LazyLock;
use tracing::debug;

use crate::mods::EntityExtension;

/// Extension record with metadata
#[derive(Debug, Clone)]
pub struct ExtensionRecord {
    pub mod_id: String,
    pub base: String,          // e.g., "legacy.animals.elephant"
    pub extension_key: String,  // e.g., "animals.elephant"
    pub extension: EntityExtension,
}

/// Global storage: extension_key -> ExtensionRecord
pub static EXTENSION_STORAGE: LazyLock<Mutex<HashMap<String, ExtensionRecord>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Also store by base for lookups: base -> extension_key
pub static EXTENSION_BY_BASE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn add_extension(
    mod_id: String,
    extension_key: String,
    extension: EntityExtension,
) -> anyhow::Result<()> {
    let base = extension.base().clone();

    // Validate: only example_tag and example_attribute supported
    for attr_key in extension.attributes().keys() {
        if attr_key != "example_attribute" {
            return Err(anyhow::anyhow!(
                "Unsupported attribute '{}': Only 'example_attribute' is currently supported",
                attr_key
            ));
        }
    }

    for tag in extension.tags().iter() {
        if tag != "example_tag" {
            return Err(anyhow::anyhow!(
                "Unsupported tag '{}': Only 'example_tag' is currently supported",
                tag
            ));
        }
    }

    {
        let mut by_base = EXTENSION_BY_BASE.lock().unwrap();
        by_base.insert(base.clone(), extension_key.clone());
    }

    let mut storage = EXTENSION_STORAGE.lock().unwrap();
    debug!(
        "Registering extension: key={}, base={}, mod={}, tags={}, attributes={}",
        extension_key,
        base,
        mod_id,
        extension.tags().len(),
        extension.attributes().len()
    );

    storage.insert(extension_key.clone(), ExtensionRecord {
        mod_id,
        base,
        extension_key,
        extension,
    });

    Ok(())
}

pub fn get_extension(extension_key: &str) -> Option<ExtensionRecord> {
    let storage = EXTENSION_STORAGE.lock().unwrap();
    storage.get(extension_key).cloned()
}

pub fn get_extension_by_base(base: &str) -> Option<ExtensionRecord> {
    let by_base = EXTENSION_BY_BASE.lock().unwrap();
    let extension_key = by_base.get(base)?.clone();
    drop(by_base);

    get_extension(&extension_key)
}

pub fn get_entity_tags(extension_key: &str) -> anyhow::Result<Vec<String>> {
    match get_extension(extension_key) {
        Some(record) => Ok(record.extension.tags().clone()),
        None => Ok(Vec::new()),
    }
}

pub fn get_entity_attribute(
    extension_key: &str,
    attribute_key: &str,
) -> anyhow::Result<Option<String>> {
    match get_extension(extension_key) {
        Some(record) => Ok(record.extension.attributes().get(attribute_key).cloned()),
        None => Ok(None),
    }
}

pub fn entity_has_tag(extension_key: &str, tag: &str) -> anyhow::Result<bool> {
    match get_extension(extension_key) {
        Some(record) => Ok(record.extension.tags().contains(&tag.to_string())),
        None => Ok(false),
    }
}

pub fn list_extensions_with_tag(tag: &str) -> Vec<String> {
    let storage = EXTENSION_STORAGE.lock().unwrap();
    storage.iter()
        .filter(|(_, record)| record.extension.tags().contains(&tag.to_string()))
        .map(|(key, _)| key.clone())
        .collect()
}

#[cfg(feature = "integration-tests")]
pub fn clear_extensions() {
    EXTENSION_STORAGE.lock().unwrap().clear();
    EXTENSION_BY_BASE.lock().unwrap().clear();
}
