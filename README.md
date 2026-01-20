# OpenZT

An enhancement DLL for **Zoo Tycoon (2001)** written in Rust. OpenZT enables more mods, bug fixes, and feature enhancements through function detouring and memory manipulation, all without modifying any original game assets.

[![Crates.io](https://img.shields.io/crates/v/openzt)](https://crates.io/crates/openzt)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

### Core Systems
- **Lua Scripting Console** - Runtime Lua execution via TCP socket (port 8080)
- **Resource Manager** - Custom file loading and modification system
- **String Registry** - Inject custom text strings into the game
- **Settings System** - Enhanced INI configuration loading
- **Expansion Pack Support** - Add custom expansions with proper UI integration

### Game Integration
- **Entity Management** - Query and manipulate game entities
- **Habitat/Exhibit Control** - Interact with exhibits and tanks
- **UI Hooks** - Access and modify UI elements
- **Game State** - Read and modify zoo statistics (cash, guests, animals, etc.)
- **World Manager** - Query world state and map information

### Bug Fixes
- Fixed crash when maintenance workers fix fences near map edge
- Fixed crash when deleting zoo walls near map edge
- Various vanilla game stability improvements

### Developer Tools
- **Integration Tests** - Automated testing in live game environment
- **Configurable Logging** - File and console logging with adjustable levels
- **Lua Macro System** - Simplified Lua function registration from Rust

## Installation

### Prerequisites
- Zoo Tycoon (2001) installed
- 32-bit Windows Rust environment (i686-pc-windows-msvc)

### Quick Start
```bash
# Clone repository
git clone https://github.com/openztcc/openzt.git
cd openzt

# Build and run
./openzt.bat run --release
```

The DLL will be copied to your Zoo Tycoon directory and the game will launch automatically.

### Running the Lua Console
```bash
./openzt.bat console
```

## Usage

### Lua Console Examples

Once OpenZT is running, connect with the console:

```lua
-- Get game information
get_date()                    -- Current in-game date
zoostats()                    -- Display zoo statistics
add_cash(10000)               -- Add $10,000 to budget

-- Entity management
get_selected_entity()         -- Get details about selected entity
sel_type()                    -- Get selected entity type info
list_entities()               -- List all entities in world
list_exhibits()               -- List all habitats/exhibits

-- Settings
get_setting("AI", "cKeeperMaxTiredness")
set_setting("AI", "cKeeperMaxTiredness", "100")
list_settings()               -- List all settings

-- UI interaction
continue()                    -- Click continue button
ui("click_continue")          -- Alternative UI call

-- Help
help()                        -- List all available functions
help("cash")                  -- Search for cash-related functions
```

### Configuration

OpenZT reads configuration from `openzt.toml` in your Zoo Tycoon directory:

```toml
[logging]
level = "info"           # trace, debug, info, warn, error
log_to_file = true       # Write to openzt.log
```

## Development

### Building

Always use `openzt.bat` for cargo actions to ensure correct toolchain and target:

```bash
# Build only
./openzt.bat build --release

# Build and run
./openzt.bat run --release

# Wait for game to exit
./openzt.bat run --release --wait

# Code quality
./openzt.bat check
./openzt.bat clippy
./openzt.bat test
```

### Running Tests

OpenZT includes integration tests that run in a live game environment:

```bash
# Run integration tests (game launches and exits automatically)
./openzt.bat run --release -- --features integration-tests

# Run tests and wait for completion
./openzt.bat run --release --wait -- --features integration-tests

# View test results
cat "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\openzt_integration_tests.log"
```

### Project Structure

```
openzt/
├── openzt/                 # Main DLL crate
│   └── src/
│       ├── lib.rs          # Entry point and initialization
│       ├── scripting.rs    # Lua scripting system
│       ├── resource_manager/  # Mod loading and resource handling
│       ├── settings/       # Game settings integration
│       └── integration_tests/  # Live game tests
├── openzt-console/         # TCP-based Lua console
├── openzt-configparser/    # INI parser crate
└── openzt.bat              # Unified build script
```

### Feature Flags

- `experimental` - Experimental features (default: enabled)
- `ini` - INI settings system (default: enabled)
- `command-console` - Legacy command console (default: enabled)
- `integration-tests` - Enable integration test framework
- `capture_ztlog` - Capture and re-log vanilla game logs

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Important**: Never commit Zoo Tycoon assets, code, configs, or decompiled content. OpenZT is a clean-room reimplementation.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Zoo Tycoon is a trademark of Microsoft Corporation
- Built with [Rust](https://www.rust-lang.org/)
- Uses [retour-rs](https://github.com/Hpmason/retour-rs) for function detouring
- Lua scripting powered by [mlua](https://github.com/mlua-rs/mlua)
