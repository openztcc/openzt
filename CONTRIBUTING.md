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

### string_registry.rs
Lets you add strings that will be read by Zoo Tycoon's BFApp::loadString, currently does not let you override existing strings
```rust
pub fn add_string_to_registry(string_val: String) -> u32 { ... }
pub fn get_string_from_registry(string_id: u32) -> Result<String, &'static str> { ... }
```

## Patterns

### structs

All structs need to be prefixed with `#[repr(C)]`, this prevents Rust from optimizing them.

```rust
#[derive(Debug)]
#[repr(C)]
pub struct UIElement {
    vftable: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_string_1: ZTString,
    string_content: ZTString,
    element_name: ZTString,
    // 25 unknown u32s
    padding: [u8; 76],
    state: UIState,
}
```

You can then use the generic functions `get_from_memory` and `save_to_memory` to read/write the structs to/from Zoo Tycoon.
`#[derive(Debug)]` is also useful as it allows you to print out the struct without defining a custom formatter.

### modules
Features are split up into modules, to add a module first create a file `my_module.rs`, add the line `mod my_module` to `lib.rs`. The module can now be used by other modules. To initiate any detours or other structures a init function should be created and called behind a feature flag in `lib.rs` as below 

```rust
if cfg!(feature = "bugfix") {
    info!("Feature 'bugfix' enabled");
    bugfix::init();
}
```

### detours
You can create a detour as follows, offset is from the start of the function (you will likely need to subtract 0x400000 from a functions address, this is only the case for detours, any other memory access should be done using the full address). `cdecl` can be replaced with `thiscall` or `stdcall`.

```rust
pub mod custom_expansion {

    #[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
    pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
        unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) };  // This calls the original function
    }
}

pub fn init() {
    unsafe { custom_expansion::init_detours().unwrap() };
}
```


### Lazy static
Currently development is ongoing in multiple independent modules, this means we don't have a central struct, instead we have global variables like below

```rust
static EXPANSION_ARRAY: Lazy<Mutex<Vec<Expansion>>> = Lazy::new(|| {
    Mutex::new(Vec::new())
});
```
The mutex is likely overkill given Zoo Tycoon is single threaded, but makes them threadsafe for future proofing.
They can be accessed using something like `let mut data_mutex = EXPANSION_ARRAY.lock().unwrap();` 

### Calling Zoo Tycoon functions
Occasionally you'll need to call a ZT function rather than just hooking calls coming from ZT. Note here that we use the full address and not an offset.

```rust
let get_element_fn: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(0x0040157d) };
let element = get_element_fn(BFUIMGR_PTR, 0x2001);
```

### Feature flags

Feature flags can be added under the `[features]` heading in `Cargo.toml`

```toml
[features]
default = ["bf_registry", "console", "ini", "ztui", "bugfix", "experimental"]
release = ["bf_registry", "ini", "ztui", "bugfix"]
console = []
ini = []
bf_registry = []
zoo_logging = []
ztui = []
bugfix = []
experimental = []
```

Features that are also listed after `default` are included by default when building. Those listed under `release` are included in release builds. To start with put your code behind the `experimental` feature flag.

To put code behind a feature flag use the `cfg!` macro
```rust
if cfg!(feature = "console") {
    info!("Feature 'console' enabled");
    zoo_console::init();
}
```