//!The ini module provides all the things necessary to load and parse ini-syntax files. The most important of which is the `Ini` struct.
//!See the [implementation](https://docs.rs/configparser/*/configparser/ini/struct.Ini.html) documentation for more details.
use std::collections::hash_map::Entry;
#[cfg(feature = "indexmap")]
use indexmap::IndexMap as Map;
#[cfg(not(feature = "indexmap"))]
use std::collections::HashMap as Map;

#[deprecated(
since = "3.0.4",
note = "async-std runtime has been replaced with tokio"
)]
#[cfg(feature = "async-std")]
#[cfg(feature = "tokio")]
use tokio::fs as async_fs;

use std::collections::HashMap;
use std::convert::AsRef;
use std::fmt::{Debug, Write};
use std::fs;
use std::path::Path;
use std::str::FromStr;

//TODO: Reintroduce multiline support

///The `Ini` struct simply contains a nested hashmap of the loaded configuration, the default section header and comment symbols.
///## Example
///```rust
///use openzt_configparser::ini::Ini;
///
///let mut config = Ini::new();
///```
#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct Ini {
    map: Map<String, Map<String, Option<Vec<String>>>>,
    default_section: std::string::String,
    comment_symbols: Vec<char>,
    delimiters: Vec<char>,
    boolean_values: HashMap<bool, Vec<String>>,
    boolean_map: HashMap<String, bool>,
    case_sensitive: bool,
    multiline: bool,
}

///The `IniDefault` struct serves as a template to create other `Ini` objects from. It can be used to store and load
///default properties from different `Ini` objects.
///## Example
///```rust
///use openzt_configparser::ini::Ini;
///
///let mut config = Ini::new();
///let default = config.defaults();
///let mut config2 = Ini::new_from_defaults(default); // default gets consumed
///```
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct IniDefault {
    ///Denotes the default section header name.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let default = config.defaults();
    ///assert_eq!(default.default_section, "default");
    ///```
    pub default_section: std::string::String,
    ///Denotes the set comment symbols for the object.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let default = config.defaults();
    ///assert_eq!(default.comment_symbols, vec![';', '#']);
    ///```
    pub comment_symbols: Vec<char>,
    ///Denotes the set delimiters for the key-value pairs.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let default = config.defaults();
    ///assert_eq!(default.delimiters, vec!['=', ':']);
    ///```
    pub delimiters: Vec<char>,
    pub boolean_values: HashMap<bool, Vec<String>>,
    ///Denotes if the `Ini` object is case-sensitive.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let default = config.defaults();
    ///assert_eq!(default.case_sensitive, false);
    ///```
    pub case_sensitive: bool,
    ///Denotes if the `Ini` object parses multiline strings.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let default = config.defaults();
    ///assert_eq!(default.multiline, false);
    ///```
    pub multiline: bool,
    pub boolean_map: HashMap<String, bool>,
}

impl Default for IniDefault {
    fn default() -> Self {
        Self {
            default_section: "default".to_owned(),
            comment_symbols: vec![';', '#'],
            delimiters: vec!['=', ':'],
            multiline: false,
            boolean_values: [
                (
                    true,
                    ["true", "yes", "t", "y", "on", "1"]
                        .iter()
                        .map(|&s| s.to_owned())
                        .collect(),
                ),
                (
                    false,
                    ["false", "no", "f", "n", "off", "0"]
                        .iter()
                        .map(|&s| s.to_owned())
                        .collect(),
                ),
            ]
                .iter()
                .cloned()
                .collect(),
            case_sensitive: false,
            boolean_map: HashMap::from([
                ("true".to_owned(), true),
                ("yes".to_owned(), true),
                ("t".to_owned(), true),
                ("y".to_owned(), true),
                ("on".to_owned(), true),
                ("1".to_owned(), true),
                ("false".to_owned(), false),
                ("no".to_owned(), false),
                ("f".to_owned(), false),
                ("n".to_owned(), false),
                ("off".to_owned(), false),
                ("0".to_owned(), false),
            ]),
        }
    }
}

/// Use this struct to define formatting options for the `pretty_write` functions.
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct WriteOptions {
    ///If true then the keys and values will be separated by " = ". In the special case where the value is empty, the
    ///line ends with " =".
    ///If false then keys and values will be separated by "=".
    ///Default is `false`.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::WriteOptions;
    ///
    ///let mut write_options = WriteOptions::default();
    ///assert_eq!(write_options.space_around_delimiters, false);
    ///```
    pub space_around_delimiters: bool,

    ///Defines the number of spaces for indentation of for multiline values.
    ///Default is 4 spaces.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::WriteOptions;
    ///
    ///let mut write_options = WriteOptions::default();
    ///assert_eq!(write_options.multiline_line_indentation, 4);
    ///```
    pub multiline_line_indentation: usize,

    ///Defines the number of blank lines between sections.
    ///Default is 0.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::WriteOptions;
    ///
    ///let mut write_options = WriteOptions::default();
    ///assert_eq!(write_options.blank_lines_between_sections, 0);
    ///```
    pub blank_lines_between_sections: usize,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            space_around_delimiters: false,
            multiline_line_indentation: 4,
            blank_lines_between_sections: 0,
        }
    }
}

impl WriteOptions {
    ///Creates a new `WriteOptions` object with the default values.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::WriteOptions;
    ///
    ///let write_options = WriteOptions::new();
    ///assert_eq!(write_options.space_around_delimiters, false);
    ///assert_eq!(write_options.multiline_line_indentation, 4);
    ///assert_eq!(write_options.blank_lines_between_sections, 0);
    ///```
    ///Returns the struct and stores it in the calling variable.
    pub fn new() -> WriteOptions {
        WriteOptions::default()
    }

    ///Creates a new `WriteOptions` object with the given parameters.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::WriteOptions;
    ///
    ///let write_options = WriteOptions::new_with_params(true, 2, 1);
    ///assert_eq!(write_options.space_around_delimiters, true);
    ///assert_eq!(write_options.multiline_line_indentation, 2);
    ///assert_eq!(write_options.blank_lines_between_sections, 1);
    ///```
    ///Returns the struct and stores it in the calling variable.
    pub fn new_with_params(
        space_around_delimiters: bool,
        multiline_line_indentation: usize,
        blank_lines_between_sections: usize,
    ) -> WriteOptions {
        Self {
            space_around_delimiters,
            multiline_line_indentation,
            blank_lines_between_sections,
        }
    }
}

#[cfg(windows)]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &str = "\n";

impl Ini {
    ///Creates a new `Map` of `Map<String, Map<String, Option<String>>>` type for the struct.
    ///All values in the Map are stored in `String` type.
    ///
    ///By default, [`std::collections::HashMap`] is used for the Map object.
    ///The `indexmap` feature can be used to use an [`indexmap::map::IndexMap`] instead, which
    ///allows keeping the insertion order for sections and keys.
    ///
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///```
    ///Returns the struct and stores it in the calling variable.
    pub fn new() -> Ini {
        Ini::new_from_defaults(IniDefault::default())
    }

    ///Creates a new **case-sensitive** `Map` of `Map<String, Map<String, Option<String>>>` type for the struct.
    ///All values in the Map are stored in `String` type.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new_cs();
    ///```
    ///Returns the struct and stores it in the calling variable.
    pub fn new_cs() -> Ini {
        Ini::new_from_defaults(IniDefault {
            case_sensitive: true,
            ..Default::default()
        })
    }

    ///Creates a new `Ini` with the given defaults from an existing `IniDefault` object.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///use openzt_configparser::ini::IniDefault;
    ///
    ///let mut default = IniDefault::default();
    ///default.comment_symbols = vec![';'];
    ///default.delimiters = vec!['='];
    ///let mut config = Ini::new_from_defaults(default.clone());
    ///// Now, load as usual with new defaults:
    ///let map = config.load("tests/test.ini").unwrap();
    ///assert_eq!(config.defaults(), default);
    ///
    ///```
    pub fn new_from_defaults(defaults: IniDefault) -> Ini {
        Ini {
            map: Map::new(),
            default_section: defaults.default_section,
            comment_symbols: defaults.comment_symbols,
            delimiters: defaults.delimiters,
            boolean_values: defaults.boolean_values,
            boolean_map: defaults.boolean_map,
            case_sensitive: defaults.case_sensitive,
            multiline: defaults.multiline,
        }
    }

    ///Fetches the defaults from the current `Ini` object and stores it as a `IniDefault` struct for usage elsewhere.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let default = config.defaults();
    ///```
    ///Returns an `IniDefault` object. Keep in mind that it will get borrowed since it has non-`Copy` types.
    pub fn defaults(&self) -> IniDefault {
        IniDefault {
            default_section: self.default_section.to_owned(),
            comment_symbols: self.comment_symbols.to_owned(),
            delimiters: self.delimiters.to_owned(),
            boolean_values: self.boolean_values.to_owned(),
            boolean_map: self.boolean_map.to_owned(),
            case_sensitive: self.case_sensitive,
            multiline: self.multiline,
        }
    }

    ///Takes an `IniDefault` object and stores its properties in the calling `Ini` object. This happens in-place and
    ///does not work retroactively, only future operations are affected.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///use openzt_configparser::ini::IniDefault;
    ///
    ///let mut config = Ini::new();
    ///let mut default = IniDefault::default();
    ///default.case_sensitive = true;
    ///// This is equivalent to ini_cs() defaults
    ///config.load_defaults(default.clone());
    ///assert_eq!(config.defaults(), default);
    ///```
    ///Returns nothing.
    pub fn load_defaults(&mut self, defaults: IniDefault) {
        self.default_section = defaults.default_section;
        self.comment_symbols = defaults.comment_symbols;
        self.delimiters = defaults.delimiters;
        self.boolean_values = defaults.boolean_values;
        self.case_sensitive = defaults.case_sensitive;
    }

    ///Sets the default section header to the defined string (the default is `default`).
    ///It must be set before `load()` or `read()` is called in order to take effect.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///
    ///config.set_default_section("topsecret");
    ///let map = config.load("tests/test.ini").unwrap();
    ///```
    ///Returns nothing.
    pub fn set_default_section(&mut self, section: &str) {
        self.default_section = section.to_owned();
    }

    ///Sets the default comment symbols to the defined character slice (the defaults are `;` and `#`).
    ///Keep in mind that this will remove the default symbols. It must be set before `load()` or `read()` is called in order to take effect.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.set_comment_symbols(&['!', '#']);
    ///let map = config.load("tests/test.ini").unwrap();
    ///```
    ///Returns nothing.
    pub fn set_comment_symbols(&mut self, symlist: &[char]) {
        self.comment_symbols = symlist.to_vec();
    }

    ///Sets multiline string support.
    ///It must be set before `load()` or `read()` is called in order to take effect.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.set_multiline(true);
    ///let map = config.load("tests/test.ini").unwrap();
    ///```
    ///Returns nothing.
    pub fn set_multiline(&mut self, multiline: bool) {
        self.multiline = multiline;
    }

    ///Gets all the sections of the currently-stored `Map` in a vector.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.load("tests/test.ini");
    ///let sections = config.sections();
    ///```
    ///Returns `Vec<String>`.
    pub fn sections(&self) -> Vec<String> {
        self.map.keys().cloned().collect()
    }

    ///Loads a file from a defined path, parses it and puts the hashmap into our struct.
    ///At one time, it only stores one configuration, so each call to `load()` or `read()` will clear the existing `Map`, if present.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let map = config.load("tests/test.ini").unwrap();  // we can get a clone like this, or just store it
    /////Then, we can use standard hashmap functions like:
    ///let values = map.get("values").unwrap();
    ///```
    ///Returns `Ok(map)` with a clone of the stored `Map` if no errors are thrown or else `Err(error_string)`.
    ///Use `get_mut_map()` if you want a mutable reference.
    pub fn load<T: AsRef<Path>>(
        &mut self,
        path: T,
    ) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        self.map = match self.parse(match fs::read_to_string(&path) {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(s) => s,
        }) {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(map) => map,
        };
        Ok(self.map.clone())
    }

    ///Loads a file from a defined path, parses it and applies it to the existing hashmap in our struct.
    ///While `load()` will clear the existing `Map`, `load_and_append()` applies the new values on top of
    ///the existing hashmap, preserving previous values.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.load("tests/test.ini").unwrap();
    ///config.load_and_append("tests/sys_cfg.ini").ok();  // we don't have to worry if this doesn't succeed
    ///config.load_and_append("tests/user_cfg.ini").ok();  // we don't have to worry if this doesn't succeed
    ///let map = config.get_map().unwrap();
    /////Then, we can use standard hashmap functions like:
    ///let values = map.get("values").unwrap();
    ///```
    ///Returns `Ok(map)` with a clone of the stored `Map` if no errors are thrown or else `Err(error_string)`.
    ///Use `get_mut_map()` if you want a mutable reference.
    pub fn load_and_append<T: AsRef<Path>>(
        &mut self,
        path: T,
    ) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        let loaded = match self.parse(match fs::read_to_string(&path) {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(s) => s,
        }) {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(map) => map,
        };

        for (section, section_map) in loaded.iter() {
            self.map
                .entry(section.clone())
                .or_default()
                .extend(section_map.clone());
        }

        Ok(self.map.clone())
    }

    ///Reads an input string, parses it and puts the hashmap into our struct.
    ///At one time, it only stores one configuration, so each call to `load()` or `read()` will clear the existing `Map`, if present.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let map = match config.read(String::from(
    ///    "[2000s]
    ///    2020 = bad")) {
    ///    Err(why) => panic!("{}", why),
    ///    Ok(inner) => inner
    ///};
    ///let this_year = map["2000s"]["2020"].clone().unwrap();
    ///assert_eq!(this_year[0], "bad"); // value accessible!
    ///```
    ///Returns `Ok(map)` with a clone of the stored `Map` if no errors are thrown or else `Err(error_string)`.
    ///Use `get_mut_map()` if you want a mutable reference.
    pub fn read(
        &mut self,
        input: String,
    ) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        self.map = match self.parse(input) {
            Err(why) => return Err(why),
            Ok(map) => map,
        };
        Ok(self.map.clone())
    }

    ///Reads an input string, parses it and applies it to the existing hashmap in our struct.
    ///While `read()` and `load()` will clear the existing `Map`, `read_and_append()` applies the new
    ///values on top of the existing hashmap, preserving previous values.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///if let Err(why) = config.read(String::from(
    ///    "[2000s]
    ///    2020 = bad
    ///    2023 = better")) {
    ///    panic!("{}", why);
    ///};
    ///if let Err(why) = config.read_and_append(String::from(
    ///    "[2000s]
    ///    2020 = terrible")) {
    ///    panic!("{}", why);
    ///};
    ///let map = config.get_map().unwrap();
    ///let few_years_ago = map["2000s"]["2020"].clone().unwrap();
    ///let this_year = map["2000s"]["2023"].clone().unwrap();
    ///assert_eq!(few_years_ago[0], "terrible"); // value updated!
    ///assert_eq!(this_year[0], "better"); // keeps old values!
    ///```
    ///Returns `Ok(map)` with a clone of the stored `Map` if no errors are thrown or else `Err(error_string)`.
    ///Use `get_mut_map()` if you want a mutable reference.
    pub fn read_and_append(
        &mut self,
        input: String,
    ) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        let loaded = match self.parse(input) {
            Err(why) => return Err(why),
            Ok(map) => map,
        };

        for (section, section_map) in loaded.iter() {
            self.map
                .entry(section.clone())
                .or_default()
                .extend(section_map.clone());
        }

        Ok(self.map.clone())
    }

    ///Writes the current configuation to the specified path using default formatting.
    ///If a file is not present then it is automatically created for you. If a file already exists then it is overwritten.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///fn main() -> std::io::Result<()> {
    ///  let mut config = Ini::new();
    ///  config.read(String::from(
    ///    "[2000s]
    ///    2020 = bad"));
    ///  config.write("output.ini")
    ///}
    ///```
    ///Returns a `std::io::Result<()>` type dependent on whether the write was successful or not.
    pub fn write<T: AsRef<Path>>(&self, path: T) -> std::io::Result<()> {
        fs::write(path.as_ref(), self.unparse(&WriteOptions::default()))
    }

    ///Writes the current configuation to the specified path using the given formatting options.
    ///If a file is not present then it is automatically created for you. If a file already exists then it is overwritten.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::{Ini, WriteOptions};
    ///
    ///fn main() -> std::io::Result<()> {
    ///  let mut write_options = WriteOptions::default();
    ///  write_options.space_around_delimiters = true;
    ///  write_options.multiline_line_indentation = 2;
    ///  write_options.blank_lines_between_sections = 1;
    ///
    ///  let mut config = Ini::new();
    ///  config.read(String::from(
    ///    "[2000s]
    ///    2020 = bad"));
    ///  config.pretty_write("output.ini", &write_options)
    ///}
    ///```
    ///Returns a `std::io::Result<()>` type dependent on whether the write was successful or not.
    pub fn pretty_write<T: AsRef<Path>>(
        &self,
        path: T,
        write_options: &WriteOptions,
    ) -> std::io::Result<()> {
        fs::write(path.as_ref(), self.unparse(write_options))
    }

    ///Returns a string with the current configuration formatted with valid ini-syntax using default formatting.
    ///This is always safe since the configuration is validated during parsing.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[2000s]
    ///  2020 = bad"));
    ///let outstring = config.writes();
    ///```
    ///Returns a `String` type contatining the ini-syntax file.
    pub fn writes(&self) -> String {
        self.unparse(&WriteOptions::default())
    }

    ///Returns a string with the current configuration formatted with valid ini-syntax using the given formatting options.
    ///This is always safe since the configuration is validated during parsing.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::{Ini, WriteOptions};
    ///
    ///let mut write_options = WriteOptions::default();
    ///write_options.space_around_delimiters = true;
    ///write_options.multiline_line_indentation = 2;
    ///write_options.blank_lines_between_sections = 1;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[2000s]
    ///  2020 = bad"));
    ///let outstring = config.pretty_writes(&write_options);
    ///```
    ///Returns a `String` type contatining the ini-syntax file.
    pub fn pretty_writes(&self, write_options: &WriteOptions) -> String {
        self.unparse(write_options)
    }

    ///Private function that converts the currently stored configuration into a valid ini-syntax string.
    fn unparse(&self, write_options: &WriteOptions) -> String {
        // push key/value pairs in outmap to out string.
        fn unparse_key_values(
            out: &mut String,
            outmap: &Map<String, Option<Vec<String>>>,
            multiline: bool,
            space_around_delimiters: bool,
            indent: usize,
        ) {
            let delimiter = if space_around_delimiters { " = " } else { "=" };
            for (key, val) in outmap.iter() {
                if let Some(value_vec) = val {
                    for value in value_vec {
                        out.push_str(key);
                        // if let Some(value) = vals {
                        if value.is_empty() {
                            out.push_str(delimiter.trim_end());
                        } else {
                            out.push_str(delimiter);
                        }

                        // if is_multiline(value) {
                        //     let mut lines = value.lines();
                        //
                        //     out.push_str(lines.next().unwrap_or_default());
                        //
                        //     for line in lines {
                        //         out.push_str(LINE_ENDING);
                        //         out.push_str(" ".repeat(indent).as_ref());
                        //         out.push_str(line);
                        //     }
                        // } else {
                        //     out.push_str(value);
                        // }
                        // }
                        out.push_str(value);
                        out.push_str(LINE_ENDING);
                    }
                    // out.push_str(LINE_ENDING);
                } else {
                    out.push_str(key);
                    out.push_str(LINE_ENDING);
                }
            }
        }

        let line_endings = LINE_ENDING.repeat(write_options.blank_lines_between_sections);
        let mut out = String::new();

        if let Some(defaultmap) = self.map.get(&self.default_section) {
            unparse_key_values(
                &mut out,
                defaultmap,
                self.multiline,
                write_options.space_around_delimiters,
                write_options.multiline_line_indentation,
            );
        }

        let mut is_first = true;
        for (section, secmap) in self.map.iter() {
            if !is_first {
                out.push_str(line_endings.as_ref());
            }
            if section != &self.default_section {
                write!(out, "[{}]", section).unwrap();
                out.push_str(LINE_ENDING);
                unparse_key_values(
                    &mut out,
                    secmap,
                    self.multiline,
                    write_options.space_around_delimiters,
                    write_options.multiline_line_indentation,
                );
            }
            is_first = false;
        }
        out
    }

    ///Private function that parses ini-style syntax into a Map.
    fn parse(&self, input: String) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        let mut map: Map<String, Map<String, Option<Vec<String>>>> = Map::new();
        let mut section = self.default_section.clone();
        let mut current_key: Option<String> = None;

        let caser = |val: &str| {
            if self.case_sensitive {
                val.to_owned()
            } else {
                val.to_lowercase()
            }
        };

        for (num, raw_line) in input.lines().enumerate() {
            let line = match raw_line.find(|c: char| self.comment_symbols.contains(&c)) {
                Some(idx) => &raw_line[..idx],
                None => raw_line,
            };

            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            match (trimmed.find('['), trimmed.rfind(']')) {
                (Some(0), Some(end)) => {
                    section = caser(trimmed[1..end].trim());

                    continue;
                }
                (Some(0), None) => {
                    return Err(format!(
                        "line {}: Found opening bracket for section name but no closing bracket",
                        num
                    ));
                }
                _ => {}
            }

            // TODO: Fix multiline support
            // if line.starts_with(char::is_whitespace) && self.multiline {
            //     let key = match current_key.as_ref() {
            //         Some(x) => x,
            //         None => {
            //             return Err(format!(
            //                 "line {}: Started with indentation but there is no current entry",
            //                 num,
            //             ))
            //         }
            //     };
            //
            //     // TODO: Fix up this valmap stuff, need to get or insert vector for value
            //     let valmap = map.entry(section.clone()).or_default();
            //
            //     let val = valmap
            //         .entry(key.clone())
            //         .or_insert_with(|| Some(Vec::new())); //TODO: Investigate, Never reached? If so, add comment
            //
            //     match val {
            //         Some(x) => {
            //             x.push_str(LINE_ENDING);
            //             x.push_str(trimmed);
            //         }
            //         None => {
            //             *val = Some(format!("{}{}", LINE_ENDING, trimmed));
            //         }
            //     }
            //
            //     continue;
            // }

            let valmap = map.entry(section.clone()).or_default();

            match trimmed.find(&self.delimiters[..]) {
                Some(delimiter) => {
                    let key = caser(trimmed[..delimiter].trim());

                    if key.is_empty() {
                        return Err(format!("line {}:{}: Key cannot be empty", num, delimiter));
                    } else {
                        current_key = Some(key.clone());

                        let value = trimmed[delimiter + 1..].trim().to_owned();

                        if valmap.contains_key(&key) {
                            match valmap.get_mut(&key).unwrap() {
                                Some(x) => { x.push(value); }
                                None => { valmap.insert(key.clone(), Some(vec![value])); }
                            }
                        } else {
                            valmap.insert(key.clone(), Some(vec![value]));
                        }
                    }
                }
                None => {
                    let key = caser(trimmed);
                    current_key = Some(key.clone());

                    valmap.insert(key, None);
                }
            }
        }

        Ok(map)
    }

    ///Private function that cases things automatically depending on the set variable.
    fn autocase(&self, section: &str, key: &str) -> (String, String) {
        if self.case_sensitive {
            (section.to_owned(), key.to_owned())
        } else {
            (section.to_lowercase(), key.to_lowercase())
        }
    }

    ///Returns a clone of the stored value from the key stored in the defined section.
    ///Unlike accessing the map directly, `get()` can process your input to make case-insensitive access *if* the
    ///default constructor is used.
    ///All `get` functions will do this automatically under the hood.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.load("tests/test.ini");
    ///let value = config.get("default", "defaultvalues").unwrap();
    ///assert_eq!(value, String::from("defaultvalues"));
    ///```
    ///Returns `Some(value)` of type `String` if value is found or else returns `None`.
    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let (section, key) = self.autocase(section, key);
        match self.map.get(&section)?.get(&key)?.clone() {
            Some(val) => Some(val[val.len() - 1].clone()),
            None => None,
        }
    }

    ///Returns a clone of the stored value vector from the key stored in the defined section.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.load("tests/test_duplicate_key.ini");
    ///let value = config.get_vec("Section", "Name").unwrap();
    ///assert_eq!(value, vec!["Value1", "Value Two", "Value 3", "Four"]);
    ///```
    ///Returns `Some(value)` of type `Vec<String>` if value is found or else returns `None`.
    pub fn get_vec(&self, section: &str, key: &str) -> Option<Vec<String>> {
        let (section, key) = self.autocase(section, key);
        self.map.get(&section)?.get(&key)?.clone()
    }


    //TODO: Make this consistent with get_bool_vec_coerce
    ///Parses the stored value from the key stored in the defined section to a `bool`. For ease of use, the function converts the type coerces a match.
    ///It attempts to case-insenstively find `true`, `yes`, `t`, `y`, `1` and `on` to parse it as `True`.
    ///Similarly it attempts to case-insensitvely find `false`, `no`, `f`, `n`, `0` and `off` to parse it as `False`.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.load("tests/test.ini");
    ///let value = config.get_bool_coerce("values", "boolcoerce").unwrap().unwrap();
    ///assert!(!value);  // value accessible!
    ///```
    ///Returns `Ok(Some(value))` of type `bool` if value is found or else returns `Ok(None)`.
    ///If the parsing fails, it returns an `Err(string)`.
    pub fn get_bool_coerce(&self, section: &str, key: &str) -> Result<Option<bool>, String> {
        let (section, key) = self.autocase(section, key);
        match self.map.get(&section) {
            Some(secmap) => match secmap.get(&key) {
                Some(val) => match val {
                    Some(inner) => {
                        let boolval = &inner[inner.len() - 1].to_lowercase()[..];
                        if self
                            .boolean_values
                            .get(&true)
                            .unwrap()
                            .iter()
                            .any(|elem| elem == boolval)
                        {
                            Ok(Some(true))
                        } else if self
                            .boolean_values
                            .get(&false)
                            .unwrap()
                            .iter()
                            .any(|elem| elem == boolval)
                        {
                            Ok(Some(false))
                        } else {
                            Err(format!(
                                "Unable to parse value into bool at {}:{}",
                                section, key
                            ))
                        }
                    }
                    None => Ok(None),
                },
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    ///Parses the stored values from the key stored in the defined section to a `bool`. For ease of use, the function converts the type coerces a match.
    ///It attempts to case-insenstively find `true`, `yes`, `t`, `y`, `1` and `on` to parse it as `True`.
    ///Similarly it attempts to case-insensitvely find `false`, `no`, `f`, `n`, `0` and `off` to parse it as `False`.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.load("tests/test_duplicate_key_more.ini");
    /// let value = config.get_bool_vec_coerce("BoolsCoerce", "Bool").unwrap().unwrap();
    /// assert_eq!(value, vec![true, false, true, false]);  // values accessible!
    ///```
    ///Returns `Ok(Some(Vec(value)))` of type `bool` if value is found or else returns `Ok(None)`.
    ///If the parsing fails, it returns an `Err(string)`.
    pub fn get_bool_vec_coerce(&self, section: &str, key: &str) -> Result<Option<Vec<bool>>, String> {
        let (section, key) = self.autocase(section, key);
        match self.map.get(&section) {
            Some(secmap) => match secmap.get(&key) {
                Some(val) => match val {
                    Some(inner) => {
                        inner.into_iter().map(|x| {
                            // let boolval = &x.to_lowercase()[..];
                            let boolval = &x.to_lowercase();
                            match self.boolean_map.get(boolval) {
                                Some(val) => {
                                    Ok(Some(*val))
                                },
                                None => Err(format!("Unable to parse value into bool")),
                            }
                        }).collect()
                    }
                    None => Ok(None),
                },
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    pub fn get_parse<T>(&self, section: &str, key: &str) -> Result<Option<T>, String>
        where
        T: FromStr,
        T::Err: Debug,
        <T as FromStr>::Err: std::fmt::Display,
    {
        let (section, key) = self.autocase(section, key);
        match self.map.get(&section) {
            Some(secmap) => match secmap.get(&key) {
                Some(val) => match val {
                    Some(inner) => match inner[inner.len() - 1].to_lowercase().parse::<T>() {
                        Err(why) => Err(why.to_string()),
                        Ok(parsed) => Ok(Some(parsed)),
                    },
                    None => Ok(None),
                },
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    pub fn get_vec_parse<T>(&self, section: &str, key: &str) -> Result<Option<Vec<T>>, String>
        where
        T: FromStr,
        T::Err: Debug,
        <T as FromStr>::Err: std::fmt::Display,
        {
        let (section, key) = self.autocase(section, key);
        match self.map.get(&section) {
            Some(secmap) => match secmap.get(&key) {
                Some(val) => match val {
                    Some(inner) => match inner.iter().map(|x| x.to_lowercase().parse::<T>()).collect() {
                        Err(why) => Err(why.to_string()),
                        Ok(parsed) => Ok(Some(parsed)),
                    },
                    None => Ok(None),
                },
                None => Ok(None),
            },
            None => Ok(None),
        }
    }


    ///Returns a clone of the `Map` stored in our struct.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key=values"));
    ///let map = config.get_map().unwrap();
    ///assert_eq!(map, *config.get_map_ref());  // the cloned map is basically a snapshot that you own
    ///```
    ///Returns `Some(map)` if map is non-empty or else returns `None`.
    ///Similar to `load()` but returns an `Option` type with the currently stored `Map`.
    pub fn get_map(&self) -> Option<Map<String, Map<String, Option<Vec<String>>>>> {
        if self.map.is_empty() {
            None
        } else {
            Some(self.map.clone())
        }
    }

    ///Returns an immutable reference to the `Map` stored in our struct.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///let mapclone = config.read(String::from
    ///  ("[topsecrets]
    ///  Valueless key")).unwrap();
    /////Think of the clone as being a snapshot at a point of time while the reference always points to the current configuration.
    ///assert_eq!(*config.get_map_ref(), mapclone);  // same as expected.
    ///```
    ///If you just need to definitely mutate the map, use `get_mut_map()` instead. Alternatively, you can generate a snapshot by getting a clone
    ///with `get_map()` and work with that.
    pub fn get_map_ref(&self) -> &Map<String, Map<String, Option<Vec<String>>>> {
        &self.map
    }

    ///Returns a mutable reference to the `Map` stored in our struct.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from
    ///  ("[topsecrets]
    ///  Valueless key"));
    /////We can then get the mutable map and insert a value like:
    ///config.get_mut_map().get_mut("topsecrets").unwrap().insert(String::from("nuclear launch codes"), None);
    ///assert_eq!(config.get("topsecrets", "nuclear launch codes"), None);  // inserted successfully!
    ///```
    ///If you just need to access the map without mutating, use `get_map_ref()` or make a clone with `get_map()` instead.
    pub fn get_mut_map(&mut self) -> &mut Map<String, Map<String, Option<Vec<String>>>> {
        &mut self.map
    }

    ///Sets an `Option<String>` in the `Map` stored in our struct. If a particular section or key does not exist, it will be automatically created.
    ///An existing value or values in the map  will be overwritten. You can also set `None` safely.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key=value"));
    ///let key_value = String::from("value");
    ///config.set("section", "key", Some(key_value));
    ///config.set("section", "key", None);  // also valid!
    ///assert_eq!(config.get("section", "key"), None);  // correct!
    ///```
    ///Returns `None` if there is no existing value, else returns `Some(Option<Vec<String>>)`, with the existing value being the wrapped `Option<String>`.
    ///If you want to insert using a string literal, use `setstr()` instead.
    pub fn set(
        &mut self,
        section: &str,
        key: &str,
        value: Option<String>,
    ) -> Option<Option<Vec<String>>> {
        let (section, key) = self.autocase(section, key);
        let value = match value {
            Some(val) => Some(vec![val]),
            None => None,
        };
        match self.map.get_mut(&section) {
            Some(secmap) => secmap.insert(key, value),
            None => {
                let mut valmap: Map<String, Option<Vec<String>>> = Map::new();
                valmap.insert(key, value);
                self.map.insert(section, valmap);
                None
            }
        }
    }

    ///Sets an `Option<&str>` in the `Map` stored in our struct. If a particular section or key does not exist, it will be automatically created.
    ///An existing value or values in the map  will be overwritten. You can also set `None` safely.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key=notvalue"));
    ///config.setstr("section", "key", Some("value"));
    ///config.setstr("section", "key", None);  // also valid!
    ///assert_eq!(config.get("section", "key"), None);  // correct!
    ///```
    ///Returns `None` if there is no existing value, else returns `Some(Option<Vec<String>>)`, with the existing value being the wrapped `Option<String>`.
    ///If you want to insert using a `String`, use `set()` instead.
    pub fn setstr(
        &mut self,
        section: &str,
        key: &str,
        value: Option<&str>,
    ) -> Option<Option<Vec<String>>> {
        let (section, key) = self.autocase(section, key);
        self.set(&section, &key, value.map(String::from))
    }

    ///Adds an `&str` to the `Map` stored in our struct. If a particular section or key does not exist, it will be automatically created.
    ///An existing vector in the map will be appended to. Passing in None is invalid
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key=value1"));
    ///config.add("section", "key", "value2".to_string());
    ///assert_eq!(config.get("section", "key"), Some("value2".to_string()));
    ///assert_eq!(config.get_vec("section", "key"), Some(vec!["value1".to_string(), "value2".to_string()]));
    ///```
    ///Returns the entire `vec` with the added string
    ///If you want to replace the existing value/s, use `set()` or `setstr()` instead.
    pub fn add(
        &mut self,
        section: &str,
        key: &str,
        value: String,
    ) -> Option<Option<Vec<String>>> {
        let (section, key) = self.autocase(section, key);
        match self.map.get_mut(&section) {
            Some(secmap) => {
                match secmap.get_mut(&key) {
                    Some(val) => {
                        match val {
                            Some(inner) => {
                                inner.push(value);
                                Some(Some(inner.clone()))
                            }
                            None => {
                                secmap.insert(key, Some(vec![value]))
                            }
                        }
                    }
                    None => {
                        secmap.insert(key, Some(vec![value]))
                    }
                }
            },
            None => {
                let mut valmap: Map<String, Option<Vec<String>>> = Map::new();
                valmap.insert(key, Some(vec![value]));
                self.map.insert(section, valmap);
                None
            }
        }
    }

    ///Adds an `&str` to the `Map` stored in our struct. If a particular section or key does not exist, it will be automatically created.
    ///An existing vector in the map will be appended to. Passing in None is invalid
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key=value1"));
    ///config.addstr("section", "key", "value2");
    ///assert_eq!(config.get("section", "key"), Some("value2".to_string()));
    ///assert_eq!(config.get_vec("section", "key"), Some(vec!["value1".to_string(), "value2".to_string()]));
    ///```
    ///Returns the entire `vec` with the added string
    ///If you want to replace the existing value/s, use `set()` or `setstr()` instead.
    pub fn addstr(
        &mut self,
        section: &str,
        key: &str,
        value: &str,
    ) -> Option<Option<Vec<String>>> {
        let (section, key) = self.autocase(section, key);
        match self.map.get_mut(&section) {
            Some(secmap) => {
                match secmap.get_mut(&key) {
                    Some(val) => {
                        match val {
                            Some(inner) => {
                                inner.push(value.to_string());
                                Some(Some(inner.clone()))
                            }
                            None => {
                                secmap.insert(key, Some(vec![value.to_string()]))
                            }
                        }
                    }
                    None => {
                        secmap.insert(key, Some(vec![value.to_string()]))
                    }
                }
            },
            None => {
                let mut valmap: Map<String, Option<Vec<String>>> = Map::new();
                valmap.insert(key, Some(vec![value.to_string()]));
                self.map.insert(section, valmap);
                None
            }
        }
    }

    ///Clears the map, removing all sections and properties from the hashmap. It keeps the allocated memory for reuse.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key=somevalue"));
    ///config.clear();
    ///assert!(config.get_map_ref().is_empty());  // our map is empty!
    ///```
    ///Returns nothing.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    ///Removes a section from the hashmap, returning the properties stored in the section if the section was previously in the map.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  updog=whatsupdog"));
    ///config.remove_section("section");  // this will return a cloned hashmap of the stored property
    ///assert!(config.get_map_ref().is_empty());  // with the last section removed, our map is now empty!
    ///```
    ///Returns `Some(section_map)` if the section exists or else, `None`.
    pub fn remove_section(&mut self, section: &str) -> Option<Map<String, Option<Vec<String>>>> {
        let section = if self.case_sensitive {
            section.to_owned()
        } else {
            section.to_lowercase()
        };
        self.map.remove(&section)
    }

    ///Removes a key from a section in the hashmap, returning the value attached to the key if it was previously in the map.
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  updog=whatsupdog
    ///  [anothersection]
    ///  updog=differentdog"));
    ///let val = config.remove_key("anothersection", "updog").unwrap().unwrap();
    ///assert_eq!(val[0], String::from("differentdog"));  // with the last section removed, our map is now empty!
    ///```
    ///Returns `Some(Option<String>)` if the value exists or else, `None`.
    pub fn remove_key(&mut self, section: &str, key: &str) -> Option<Option<Vec<String>>> {
        let (section, key) = self.autocase(section, key);
        self.map.get_mut(&section)?.remove(&key)
    }

    ///Gets all the keys from a section in the Ini file
    ///## Example
    ///```rust
    ///use openzt_configparser::ini::Ini;
    ///
    ///let mut config = Ini::new();
    ///config.read(String::from(
    ///  "[section]
    ///  key1=value1
    ///  key_without_string"));
    ///let keys = config.get_keys("section").unwrap();
    ///assert_eq!(keys.contains(&"key1".to_string()), true);
    ///assert_eq!(keys.contains(&"key_without_string".to_string()), true);
    ///```
    ///Returns `Some(Vec<String>)` if the section exists, else `None`.
    pub fn get_keys(&mut self, section: &str) -> Option<Vec<String>> {
        match self.map.get(section) {
            Some(secmap) => {
                let mut keys = Vec::new();
                for (key, _) in secmap.iter() {
                    keys.push(key.clone());
                }
                Some(keys)
            },
            None => None,
        }
    }
}

#[cfg(feature = "async-std")]
impl Ini {
    ///Loads a file asynchronously from a defined path, parses it and puts the hashmap into our struct.
    ///At one time, it only stores one configuration, so each call to `load()` or `read()` will clear the existing `Map`, if present.
    ///
    ///Usage is similar to `load`, but `.await` must be called after along with the usual async rules.
    ///
    ///Returns `Ok(map)` with a clone of the stored `Map` if no errors are thrown or else `Err(error_string)`.
    ///Use `get_mut_map()` if you want a mutable reference.
    pub async fn load_async<T: AsRef<Path>>(
        &mut self,
        path: T,
    ) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        self.map = match self.parse(match async_fs::read_to_string(&path).await {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(s) => s,
        }) {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(map) => map,
        };
        Ok(self.map.clone())
    }

    ///Loads a file from a defined path, parses it and applies it to the existing hashmap in our struct.
    ///While `load_async()` will clear the existing `Map`, `load_and_append_async()` applies the new values on top
    ///of the existing hashmap, preserving previous values.
    ///
    ///Usage is similar to `load_and_append`, but `.await` must be called after along with the usual async rules.
    ///
    ///Returns `Ok(map)` with a clone of the stored `Map` if no errors are thrown or else `Err(error_string)`.
    ///Use `get_mut_map()` if you want a mutable reference.
    pub async fn load_and_append_async<T: AsRef<Path>>(
        &mut self,
        path: T,
    ) -> Result<Map<String, Map<String, Option<Vec<String>>>>, String> {
        let loaded = match self.parse(match async_fs::read_to_string(&path).await {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(s) => s,
        }) {
            Err(why) => {
                return Err(format!(
                    "couldn't read {}: {}",
                    &path.as_ref().display(),
                    why
                ));
            }
            Ok(map) => map,
        };

        for (section, section_map) in loaded.iter() {
            self.map
                .entry(section.clone())
                .or_insert_with(Map::new)
                .extend(section_map.clone());
        }

        Ok(self.map.clone())
    }

    ///Writes the current configuation to the specified path asynchronously using default formatting. If a file is not present, it is automatically created for you, if a file already
    ///exists, it is truncated and the configuration is written to it.
    ///
    ///Usage is the same as `write`, but `.await` must be called after along with the usual async rules.
    ///
    ///Returns a `std::io::Result<()>` type dependent on whether the write was successful or not.
    pub async fn write_async<T: AsRef<Path>>(&self, path: T) -> std::io::Result<()> {
        async_fs::write(path.as_ref(), self.unparse(&WriteOptions::default())).await
    }

    ///Writes the current configuation to the specified path asynchronously using the given formatting options. If a file is not present, it is automatically created for you, if a file already
    ///exists, it is truncated and the configuration is written to it.
    ///
    ///Usage is the same as `pretty_pretty_write`, but `.await` must be called after along with the usual async rules.
    ///
    ///Returns a `std::io::Result<()>` type dependent on whether the write was successful or not.
    pub async fn pretty_write_async<T: AsRef<Path>>(
        &self,
        path: T,
        write_options: &WriteOptions,
    ) -> std::io::Result<()> {
        async_fs::write(path.as_ref(), self.unparse(write_options)).await
    }
}

///Private function to check if string contains newlines
fn is_multiline(line: &String) -> bool {
    line.contains(LINE_ENDING)
}
