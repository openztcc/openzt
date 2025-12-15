use std::{collections::HashMap, error::Error, fmt, str::FromStr};

use getset::Getters;
use indexmap::IndexMap;
use serde::{
    de::{self, Deserializer, Visitor},
    Deserialize,
};

#[derive(Debug)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    pub fn new(message: String) -> Self {
        ParseError { message }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParseError: {}", self.message)
    }
}

impl Error for ParseError {}

impl From<std::num::ParseIntError> for ParseError {
    fn from(err: std::num::ParseIntError) -> Self {
        ParseError {
            message: format!("Failed to parse int: {}", err),
        }
    }
}

#[derive(Deserialize, Debug, Getters, Clone)]
#[get = "pub"]
pub struct Meta {
    name: String,
    description: String,
    authors: Vec<String>,
    mod_id: String,
    #[serde(deserialize_with = "deserialize_version")]
    version: Version,
    #[serde(default = "default_ztd_type")]
    ztd_type: ZtdType,
    link: Option<String>,
    #[serde(default = "default_empty_dependencies")]
    dependencies: Vec<Dependencies>,
}

fn default_ztd_type() -> ZtdType {
    ZtdType::Openzt
}

fn default_empty_dependencies() -> Vec<Dependencies> {
    Vec::new()
}

#[derive(Deserialize, Default, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ZtdType {
    Legacy,
    Combined,
    #[default]
    Openzt,
}

#[derive(Debug, PartialEq, Clone, Getters)]
#[get = "pub"]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(ParseError::new(format!("Invalid version string: {} (expected 'x.y.z' e.g '1.0.0')", s)));
        }

        Ok(Version {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
        })
    }
}

fn deserialize_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    struct VersionVisitor;

    impl<'de> Visitor<'de> for VersionVisitor {
        type Value = Version;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a version string in the format 'x.y.z'")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Version::from_str(value).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_str(VersionVisitor)
}

fn deserialize_version_option<'de, D>(deserializer: D) -> Result<Option<Version>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionVersionVisitor;

    impl<'de> Visitor<'de> for OptionVersionVisitor {
        type Value = Option<Version>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a version string in the format 'x.y.z'")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match Version::from_str(value) {
                Ok(v) => Ok(Some(v)),
                Err(err) => Err(de::Error::custom(err)),
            }
        }
    }

    deserializer.deserialize_str(OptionVersionVisitor)
}

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize, Clone, Debug, Getters)]
#[get = "pub"]
pub struct Dependencies {
    mod_id: String,
    name: String,
    #[serde(deserialize_with = "deserialize_version_option")]
    min_version: Option<Version>,
    #[serde(default = "default_as_false")]
    optional: bool,
    #[serde(default)]
    ordering: Ordering,
}

#[derive(Deserialize, Default, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Ordering {
    Before,
    After,
    #[default]
    None,
}

#[derive(Deserialize, Debug, Getters)]
#[get = "pub"]
pub struct ModDefinition {
    habitats: Option<HashMap<String, IconDefinition>>,
    locations: Option<HashMap<String, IconDefinition>>,
}

impl ModDefinition {
    pub fn len(&self) -> usize {
        let mut len = 0;
        if let Some(habitats) = &self.habitats {
            len += habitats.len();
        }
        if let Some(locations) = &self.locations {
            len += locations.len();
        }
        len
    }
}

#[derive(Deserialize, Debug, Getters)]
#[get = "pub"]
pub struct IconDefinition {
    name: String,
    icon_path: String,
    icon_palette_path: String,
}

// ============================================================================
// Patch System Data Structures
// ============================================================================

#[derive(Deserialize, Debug, Clone)]
pub struct PatchFile {
    /// Top-level on_error directive for file-level error handling
    #[serde(default = "default_on_error")]
    pub on_error: ErrorHandling,

    /// Top-level conditions - if these fail, entire file is skipped
    #[serde(default)]
    pub condition: Option<PatchCondition>,

    /// Named patches (preserves insertion order via IndexMap)
    pub patches: IndexMap<String, Patch>,
}

fn default_on_error() -> ErrorHandling {
    ErrorHandling::Continue
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorHandling {
    Continue,
    Abort,
    AbortMod,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Patch {
    Replace(ReplacePatch),
    Merge(MergePatch),
    Delete(DeletePatch),
    SetKey(SetKeyPatch),
    SetKeys(SetKeysPatch),
    AppendValue(AppendValuePatch),
    AppendValues(AppendValuesPatch),
    RemoveKey(RemoveKeyPatch),
    RemoveKeys(RemoveKeysPatch),
    AddSection(AddSectionPatch),
    ClearSection(ClearSectionPatch),
    RemoveSection(RemoveSectionPatch),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MergeMode {
    PatchPriority,
    BasePriority,
}

fn default_merge_mode() -> MergeMode {
    MergeMode::PatchPriority
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnExists {
    Error,
    Merge,
    Skip,
    Replace,
}

fn default_on_exists() -> OnExists {
    OnExists::Error
}

#[derive(Deserialize, Debug, Clone)]
pub struct PatchCondition {
    #[serde(default)]
    pub mod_loaded: Option<String>,
    #[serde(default)]
    pub key_exists: Option<KeyCheck>,
    #[serde(default)]
    pub value_equals: Option<ValueCheck>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyCheck {
    pub section: String,
    pub key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ValueCheck {
    pub section: String,
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReplacePatch {
    pub target: String,
    pub source: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MergePatch {
    pub target: String,
    pub source: String,
    #[serde(default = "default_merge_mode")]
    pub merge_mode: MergeMode,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeletePatch {
    pub target: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SetKeyPatch {
    pub target: String,
    pub section: String,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SetKeysPatch {
    pub target: String,
    pub section: String,
    pub keys: HashMap<String, String>,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppendValuePatch {
    pub target: String,
    pub section: String,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppendValuesPatch {
    pub target: String,
    pub section: String,
    pub key: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemoveKeyPatch {
    pub target: String,
    pub section: String,
    pub key: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemoveKeysPatch {
    pub target: String,
    pub section: String,
    pub keys: Vec<String>,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddSectionPatch {
    pub target: String,
    pub section: String,
    #[serde(default)]
    pub keys: HashMap<String, String>,
    #[serde(default = "default_on_exists")]
    pub on_exists: OnExists,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClearSectionPatch {
    pub target: String,
    pub section: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RemoveSectionPatch {
    pub target: String,
    pub section: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[cfg(test)]
mod mod_loading_tests {
    use crate::mods::Version;

    #[test]
    fn test_parse_meta() {
        let meta: super::Meta = toml::from_str(include_str!("../resources/test/meta.toml")).unwrap();
        assert_eq!(meta.name, "my fun mod");
        assert_eq!(meta.description, "a mod full of fun");
        assert_eq!(meta.authors, vec!["Finn".to_string()]);
        assert_eq!(meta.mod_id, "finn.my_fun_mod");
        assert_eq!(meta.version, Version { major: 1, minor: 0, patch: 0 });
        assert_eq!(meta.version.minor, 0);
        assert_eq!(meta.version.patch, 0);
        assert_eq!(meta.link, Some("https://mywebsite.com/myfunmod".to_string()));
        assert_eq!(meta.dependencies.len(), 1);
        assert_eq!(meta.ztd_type, super::ZtdType::Openzt);
        let dep = meta.dependencies[0].clone();
        assert_eq!(dep.mod_id, "finn.my_other_mod");
        assert_eq!(dep.name, "my other mod");
        assert_eq!(dep.min_version.unwrap(), Version { major: 1, minor: 1, patch: 2 });
        assert!(dep.optional);
        assert_eq!(dep.ordering, super::Ordering::Before);
    }

    #[test]
    fn test_parse_meta_legacy() {
        let meta: super::Meta = toml::from_str(include_str!("../resources/test/meta-legacy.toml")).unwrap();
        assert_eq!(meta.name, "my fun mod");
        assert_eq!(meta.description, "a mod full of fun");
        assert_eq!(meta.authors, vec!["Finn".to_string()]);
        assert_eq!(meta.mod_id, "finn.my_fun_mod");
        assert_eq!(meta.version, Version { major: 1, minor: 0, patch: 0 });
        assert_eq!(meta.version.minor, 0);
        assert_eq!(meta.version.patch, 0);
        assert_eq!(meta.link, Some("https://mywebsite.com/myfunmod".to_string()));
        assert_eq!(meta.ztd_type, super::ZtdType::Legacy);
    }

    fn check_moon_location(location: &super::IconDefinition) {
        assert_eq!(location.name, "Moon");
        assert_eq!(location.icon_path, "resources/moon/N");
        assert_eq!(location.icon_palette_path, "resources/moon/moon.pal");
    }

    fn check_swamp_habitat(habitat: &super::IconDefinition) {
        assert_eq!(habitat.name, "Swamp");
        assert_eq!(habitat.icon_path, "resources/swamp/N");
        assert_eq!(habitat.icon_palette_path, "resources/swamp/swamp.pal");
    }

    #[test]
    fn test_parse_location() {
        let mods: super::ModDefinition = toml::from_str(include_str!("../resources/test/example-location.toml")).unwrap();
        let locations = mods.locations.unwrap();
        let location = locations.get("moon").unwrap();
        check_moon_location(location);
    }

    #[test]
    fn test_parse_habitat() {
        let mods: super::ModDefinition = toml::from_str(include_str!("../resources/test/example-habitat.toml")).unwrap();
        let habitats = mods.habitats.unwrap();
        let habitat = habitats.get("swamp").unwrap();
        check_swamp_habitat(habitat);
    }

    #[test]
    fn test_parse_combined() {
        let mods: super::ModDefinition = toml::from_str(include_str!("../resources/test/example-combined.toml")).unwrap();
        let locations = mods.locations.unwrap();
        let habitats = mods.habitats.unwrap();
        let location = locations.get("moon").unwrap();
        check_moon_location(location);
        let location_2 = locations.get("moon2").unwrap();
        check_moon_location(location_2);
        let habitat = habitats.get("swamp").unwrap();
        check_swamp_habitat(habitat);
    }

    #[test]
    fn test_parse_patches() {
        let patches: super::PatchFile = toml::from_str(include_str!("../resources/test/patch.toml")).unwrap();
        assert_eq!(patches.patches.len(), 8);

        // Test file-level config
        assert_eq!(patches.on_error, super::ErrorHandling::Continue);
        assert!(patches.condition.is_none());

        // Test merge patch (defaults to patch_priority)
        let merge_patch = patches.patches.get("merge_blackbuck_ai").expect("merge_blackbuck_ai patch not found");
        match merge_patch {
            super::Patch::Merge(patch) => {
                assert_eq!(patch.target, "animals/blckbuck.ai");
                assert_eq!(patch.source, "resources/patches/blckbuck.ai");
                assert_eq!(patch.merge_mode, super::MergeMode::PatchPriority);
                assert!(patch.condition.is_none());
            }
            _ => panic!("Expected Merge patch"),
        }

        // Test replace patch
        let replace_patch = patches.patches.get("replace_blackbuck_ai").expect("replace_blackbuck_ai patch not found");
        match replace_patch {
            super::Patch::Replace(patch) => {
                assert_eq!(patch.target, "animals/blckbuck.ai");
                assert_eq!(patch.source, "resources/patches/blckbuck.ai");
                assert!(patch.condition.is_none());
            }
            _ => panic!("Expected Replace patch"),
        }

        // Test delete patch
        let delete_patch = patches.patches.get("remove_old_animal").expect("remove_old_animal patch not found");
        match delete_patch {
            super::Patch::Delete(patch) => {
                assert_eq!(patch.target, "animals/oldanimal.ai");
                assert!(patch.condition.is_none());
            }
            _ => panic!("Expected Delete patch"),
        }

        // Test set_key patch
        let set_key_patch = patches.patches.get("update_resolution").expect("update_resolution patch not found");
        match set_key_patch {
            super::Patch::SetKey(patch) => {
                assert_eq!(patch.target, "config/settings.ini");
                assert_eq!(patch.section, "Graphics");
                assert_eq!(patch.key, "Resolution");
                assert_eq!(patch.value, "1920x1080");
                assert!(patch.condition.is_none());
            }
            _ => panic!("Expected SetKey patch"),
        }

        // Test conditional patch with mod_loaded
        let conditional_patch = patches.patches.get("conditional_mod_loaded").expect("conditional_mod_loaded patch not found");
        match conditional_patch {
            super::Patch::SetKey(patch) => {
                assert_eq!(patch.target, "config/settings.ini");
                assert_eq!(patch.section, "Features");
                assert_eq!(patch.key, "AdvancedAI");
                assert_eq!(patch.value, "true");
                let condition = patch.condition.as_ref().expect("Expected condition");
                assert_eq!(condition.mod_loaded, Some("AdvancedAI".to_string()));
                assert!(condition.key_exists.is_none());
                assert!(condition.value_equals.is_none());
            }
            _ => panic!("Expected SetKey patch"),
        }

        // Test conditional patch with key_exists
        let conditional_patch = patches.patches.get("conditional_key_exists").expect("conditional_key_exists patch not found");
        match conditional_patch {
            super::Patch::SetKey(patch) => {
                assert_eq!(patch.target, "config/settings.ini");
                let condition = patch.condition.as_ref().expect("Expected condition");
                assert!(condition.mod_loaded.is_none());
                let key_check = condition.key_exists.as_ref().expect("Expected key_exists");
                assert_eq!(key_check.section, "Graphics");
                assert_eq!(key_check.key, "AntiAliasing");
                assert!(condition.value_equals.is_none());
            }
            _ => panic!("Expected SetKey patch"),
        }

        // Test conditional patch with value_equals
        let conditional_patch = patches.patches.get("conditional_value_equals").expect("conditional_value_equals patch not found");
        match conditional_patch {
            super::Patch::SetKey(patch) => {
                assert_eq!(patch.target, "config/settings.ini");
                let condition = patch.condition.as_ref().expect("Expected condition");
                assert!(condition.mod_loaded.is_none());
                assert!(condition.key_exists.is_none());
                let value_check = condition.value_equals.as_ref().expect("Expected value_equals");
                assert_eq!(value_check.section, "Graphics");
                assert_eq!(value_check.key, "TextureQuality");
                assert_eq!(value_check.value, "medium");
            }
            _ => panic!("Expected SetKey patch"),
        }

        // Test add_section patch with on_exists
        let add_section_patch = patches.patches.get("add_section_with_on_exists").expect("add_section_with_on_exists patch not found");
        match add_section_patch {
            super::Patch::AddSection(patch) => {
                assert_eq!(patch.target, "config/settings.ini");
                assert_eq!(patch.section, "NewFeature");
                assert_eq!(patch.on_exists, super::OnExists::Merge);
                assert_eq!(patch.keys.get("Enabled"), Some(&"true".to_string()));
                assert_eq!(patch.keys.get("Value"), Some(&"100".to_string()));
                assert!(patch.condition.is_none());
            }
            _ => panic!("Expected AddSection patch"),
        }

        // Test that patches are ordered correctly (IndexMap preserves insertion order)
        let patch_names: Vec<_> = patches.patches.keys().collect();
        assert_eq!(patch_names[0], "merge_blackbuck_ai");
        assert_eq!(patch_names[1], "replace_blackbuck_ai");
        assert_eq!(patch_names[2], "remove_old_animal");
        assert_eq!(patch_names[3], "update_resolution");
        assert_eq!(patch_names[4], "conditional_mod_loaded");
        assert_eq!(patch_names[5], "conditional_key_exists");
        assert_eq!(patch_names[6], "conditional_value_equals");
        assert_eq!(patch_names[7], "add_section_with_on_exists");
    }
}
