# Contributing

## Assets

Do not commit any assets from Zoo Tycoon, this includes config files and decompiled code. OpenZT is a complete reimplementation and does not use any code or assets from the original.

## Project layout and import modules

### lib.rs
This is the main file that handles initialising other modules and features flags.

### console.rs
Handles sending and receiving data to/from [openzt-console](https://github.com/openztcc/openzt-console), other modules can register commands via `add_to_command_register(command_name: String, command_callback: CommandCallback)` where CommandCallback looks like `type CommandCallback = fn(args: Vec<&str>) -> Result<String, &'static str>;`

### resource_mgr.rs
Handles walking through the directories listed in `zoo.ini` and extracting all files. You can register handler functions based on file prefixes and suffixes via `add_handler(handler: Handler)` and `Handler::new(matcher_prefix: Option<String>, matcher_suffix: Option<String>, handler: HandlerFunction)` where the HandlerFunction `pub type HandlerFunction = fn(&PathBuf, &mut ZipFile) -> ();`

## Patterns


### TODO: brief detour overview


### structs

All structs need to be prefixed with `#[repr(C)]`, this prevents Rust from optimizing them.

```rust
#[derive(Debug)]
#[repr(C)]
pub struct Expansion {
    expansion_id: u32,
    name_id: u32,
    name_string: ZTString,
}
```

You can then use the generic functions `get_from_memory` and `save_to_memory` to read/write the structs to/from Zoo Tycoon.

### modules
Features are split up into modules, to add a module first create a file `my_module.rs`, add the line `mod my_module` to `lib.rs`. The module can now be used by other modules. To initiate any detours or other structures a init function should be created and called behind a feature flag in `lib.rs` as below 

```rust
if cfg!(feature = "bugfix") {
    info!("Feature 'bugfix' enabled");
    bugfix::init();
}
```


### TODO: Lazy static, why we need and how to do


