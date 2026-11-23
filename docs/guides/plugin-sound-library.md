# The plugin's sound library

The CLAP/VST3 plugin plays the programs of a Nord Stage 4 backup (`.ns4b`): the factory restore file from Nord, or your own backup with the pianos and sample banks it uses. The editor's OPEN .NS4B button does that by hand; this page is the hands-free way, for a DAW that cannot show the plugin window (SoundCraft on Windows) and for projects that must sound the same when reopened.

## 1. Save the library once

```bash
ns4 library set "F:\path\to\Nord Stage 4 Factory Restore RevK v1.66.ns4b"
ns4 library status
```

`set` opens the file and says how many programs it holds; `clear` forgets it. Only the path is stored, in `ns4/config.json` under your user config folder next to the Nord OS update setting (`ns4 firmware set`). The environment variable `NS4_LIBRARY` overrides the saved path.

Give the plugin the Nord OS update too (`ns4 firmware set`, see [Where to get the Nord OS update](nord-os-update.md)): WAVE sounds then use the real wavetables.

## 2. Pick a program in the DAW

Every instance has a host parameter **Program**: `Off`, then `A:11` .. `H:88` (the locations the keyboard shows). A new instance starts at `A:11`, the first program of the library. Change it from the DAW's parameter list or an automation lane; the plugin decodes that program with its piano and sample banks on a worker thread and switches to it at the next audio block. Large pianos take a few seconds to decode.

The value is saved with the project, so a reopened project loads the same program again. `Off` plays the panel / host parameters alone, as a plugin without a library does.

## What to expect

- Adding the instrument, or opening a project, waits a few seconds while the program's banks decode (a factory grand piano is 218 MB); instances of the same program share one decode. `NS4_LIBRARY=none` in the host's environment switches the library off for that run.

- No library saved, or the file moved: the plugin starts and plays its built-in sounds; `ns4 library status` says why.
- What the plugin loaded, and why a load failed, is appended to `plugin.log` next to the settings file (`%APPDATA%\ns4\plugin.log` on Windows).
- The DAW must be running the plugin (playback or monitoring on) for a load to apply; a load waits up to 30 seconds for that.
- A program whose piano or sample bank is not in the library plays without that layer, like the keyboard's "Piano not found".
- Moving a knob of a group (organ, synth, ...) after a load gives that group back to the host parameters, as in the editor (D218).
