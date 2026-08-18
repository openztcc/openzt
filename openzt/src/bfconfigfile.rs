//! `BFConfigFile` is the vanilla base class for anything that loads itself from a `.cfg`-style INI
//! block via `getInt`/`getFloat`/`getString`/`getStringList` (e.g. `ZTResearchProgram`,
//! `ZTResearchCategory`, `ZTResearchBranch` - see `private/resources/decompiles/BFConfigFile_*` and
//! `private/resources/decompiles/ZTResearch*_load*`). Those classes inherit it directly (`this` gets cast
//! straight to `BFConfigFile*` when calling its methods), so it occupies their first `0xc` bytes.
//!
//! Only the raw layout is modeled here, not `parse`/`getInt`/etc. themselves - callers that already
//! have a live object just need to know these bytes aren't theirs to interpret.

use std::fmt;

/// Confirmed via `private/resources/decompiles/BFConfigFile_BFConfigFile.c`/`_attempt.c`/`_parse.c` and
/// cross-checked against every `ZTResearch*::load*` function that inherits it.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct BFConfigFile {
    tree_root: u32,       // 0x0 - an intrusive rb-tree root holding the parsed config blocks; not meaningful to callers
    loaded: i32,          // 0x4 - "has data" flag, set once `parse`/`addBlock` populates the tree; a freshly-constructed (not yet `attempt`-ed) instance has this at 0
    kind_tag: u8,         // 0x8 - one byte written by `BFConfigFile::BFConfigFile`'s constructor from a caller-supplied parameter (e.g. always 6 for `ZTResearchProgram`); presumably identifies what kind of thing owns this config
    pad_kind_tag: [u8; 3], // 0x9 - the rest of that same word; never written by any code we've seen, so it's leftover/uninitialized allocator memory, not meaningful
}

impl BFConfigFile {
    pub fn is_loaded(&self) -> bool {
        self.loaded != 0
    }

    pub fn kind_tag(&self) -> u8 {
        self.kind_tag
    }
}

impl fmt::Display for BFConfigFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BFConfigFile {{ loaded: {}, kind_tag: {} }}", self.is_loaded(), self.kind_tag)
    }
}
