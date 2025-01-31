use std::{collections::HashMap, error::Error, fmt, str::FromStr};

use getset::Getters;
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
}
