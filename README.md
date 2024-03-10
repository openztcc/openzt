# OpenZT

A DLL for injection into [Zoo Tycoon (2001)](https://en.wikipedia.org/wiki/Zoo_Tycoon_(2001_video_game)). Details on what classes and functions are currently patched are below.

## Overview

### Legend
✅ = Reimplemented
✅/⏳ = Partially reimplemented
⏳ = In progress

### BFRegistry ✅
This class appears to function as a mapping to constructors, potentially allowing the **[mgr]** section of **zoo.ini** to set which constructors are used at runtime. However only the **terrainmgr** field is loaded from the ini and only one option is loaded into the BFRegistry, making the class appear redundant.

### Load INI ✅/⏳

Loads settings from the **[debug]** section of **zoo.ini** and saves them into static memory addresses. Patching other ini section loading causes crashes as it was done before any OO patterns were uncovered.

### ZTUI::expansionselect::setup ✅/⏳
Loads existing expansions and supports adding new expansions via code, only outstanding feature is updating the UI dropdown to fit any new expansions.

## Contributing

OpenZT welcomes contribution from everyone. See [CONTRIBUTING.md](CONTRIBUTING.md) for help getting started.