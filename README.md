# openzt

Simple proof of concept DLL for injection into [Zoo Tycoon (2001)](https://en.wikipedia.org/wiki/Zoo_Tycoon_(2001_video_game)). Currently just patches a single call to a function that loads some debug settings from the zoo.ini file. Further reverse engineering of the class structure is required to patch anythign else (WIP).

## Classes

This will be a rundown of classes as they are reverse engineered

### Legend
✅ = Reimplemented
✅/⏳ = Partially reimplemented
⏳ = In progress

### BFRegistry ✅
This class appears to function as a mapping to constructors, potentially allowing the **[mgr]** section of **zoo.ini** to set which constructors are used at runtime. However only the **terrainmgr** field is loaded from the ini and only one option is loaded into the BFRegistry, making the class appear redundant.