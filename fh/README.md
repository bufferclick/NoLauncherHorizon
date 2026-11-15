# Fabric n Horizon

<p align="center"><img src="icons/fh-256.png" width="96" alt="Fabric n Horizon's icon: a white F and H on a black rounded tile"></p>

**[Download FH.zip](https://github.com/bufferclick/NoLauncherHorizon/releases)** — unzip it and
double-click `FH.exe` (Windows x64).

Your Horizon instances, one page. Double-click it and your browser opens on a page drawn like
**JLac**'s window (the Prism Launcher fork this one is modelled on): the bar across the top with
the mark and the signed-in account, the instances down the left in a darker pane, the chosen one
on the right with one lime **Launch** button.

1. Press Launch on an instance.
2. If this computer has no signed-in Microsoft account that owns Minecraft: Java Edition, the page
   shows the sign-in's code (in the clipboard, with a Copy button, and as a QR code for a phone)
   and Microsoft's own page opens once with the code already filled in.
3. The page shows the work: the build the instance plays is installed if this computer has no
   copy, and its resources are prepared from your own Minecraft exactly as the launcher's Play
   does (Mojang's own files are only downloaded when no install has them).
4. Horizon starts in that instance, and the tab says so: close it whenever you like.

Instances that play **Minecraft: Java Edition** are listed too, with their button off — FH starts
Horizon instances, and Minecraft ones (Fabric included) are next. Nothing is created, renamed or
deleted here: FH shows the instances the Horizon launcher has and starts one.

## Why it looks like this

The window is a page, not a toolkit: `ui.rs` draws HTML with JLac's own palette (window `#313131`,
base `#222222`, alternate `#2a2a2a`, the lime highlight `#96db59` with black text on it, teal
links), `server.rs` answers the browser on loopback, and `job.rs` does the launch. There is no
webview, no framework and no dependency that is not already in the launcher's own tree.

The page's addresses carry a word made fresh every run (`?k=…`), and the server listens only on
`127.0.0.1` and refuses anything without it — so a page on the internet cannot ask FH to start a
game.

## What it shares, and where it keeps things

The data folder is the launcher's own — on Windows `%APPDATA%\horizon-launcher`, overridden by
`HORIZON_LAUNCHER_HOME` — so instances, builds, worlds, options and the sign-in are the same ones
a launcher installed beside it uses, and Minecraft files are taken from the same places. `fh/`
inside it holds only this program's own file:

| | |
|---|---|
| `fh/fh.log` | every start and launch, with the second they came in at: the only place a failure is explained in full |

The instance you press Launch on is the one that starts (its folder, its `pre-launch` and
wrapper), and its play time is saved the way the launcher saves it.

## Options

```
FH [options]

  --version, -V    Print the version and stop
  --help, -h       Print this and stop
```

FH closes itself when the page has not been asked for anything for half an hour and no launch is
waiting on a game.

## Versions

`fh/VERSION` is the one place a version lives (`build.rs` puts it in the .exe and in `--version`,
`fh/Cargo.toml` must agree — a test checks it), and the release workflow tags the build from it:
`fh-v1.2`, `fh-v1.3`, …

One tenth per update, the same rule as NoLauncherHorizon: **v1.2** first, then v1.3 … v1.9, then
**v1.10**, never v2.0.

## Building it

```
cd fh
cargo build --release          # target/release/FH.exe
cargo test --release           # the page, the server, the rows, the version
python3 icons/make_icon.py     # icons/fh.ico from icons/fh.svg (--png 256 to look at it)
```

`.github/workflows/fh.yml` builds it on Windows, checks `--version`, wraps the .exe and
`pack/README.txt` into `FH.zip`, uploads it as an artifact and — on `main` — publishes it as the
release `fh-v<VERSION>` (never marked latest: the game's own releases stay the repository's
latest).

`fh/` is its own Cargo workspace so the launcher's workspace and lock file are untouched; it
repeats the launcher's `[patch.crates-io]` entries for wgpu-hal, naga and libmimalloc-sys,
because it links the launcher's code and those copies are the ones Horizon ships with.

MIT, like Horizon. Not an official Minecraft product; not approved by or associated with Mojang or
Microsoft. Drawn after JLac, Buffer's Prism Launcher fork.
