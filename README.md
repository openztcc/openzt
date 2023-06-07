# openzt

Very basic DLL for injection into [Zoo Tycoon (2001)](https://en.wikipedia.org/wiki/Zoo_Tycoon_(2001_video_game)). Details on what classes and functions are currently patched are below. Further reverse engineering of the class structure is required to progress (WIP).

## Classes and functions

This will be a rundown of classes as they discovered and reimplemented.

### Legend
✅ = Reimplemented
✅/⏳ = Partially reimplemented
⏳ = In progress

### BFRegistry ✅
This class appears to function as a mapping to constructors, potentially allowing the **[mgr]** section of **zoo.ini** to set which constructors are used at runtime. However only the **terrainmgr** field is loaded from the ini and only one option is loaded into the BFRegistry, making the class appear redundant.

### Load INI ✅/⏳

Loads settings from the **[debug]** section of **zoo.ini** and saves them into static memory addresses. Patching other ini section loading causes crashes as it was done before any OO patterns were uncovered.
