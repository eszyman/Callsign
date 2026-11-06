# Callsign

Callsign is a desktop companion for **IL-2 Sturmovik** (Great Battles, including Korea). It sits beside the simulator. Press a button and speak. It can type a chat line, or turn a short phrase into the keys the sim already uses for the radio, gear, flaps, and the rest of the cockpit.

It is not an overlay inside the headset. The microphone is the PC input (a headset mic that the operating system can see works, including some VR mics). Cue tones play on the default output, which SteamVR usually mirrors into the headset.

Key output starts **off**. Until you turn it on in Setup, Callsign only shows what it would have typed.

## The gesture

Scroll Lock is the default talk key, because IL-2 rarely wants it. A joystick or HOTAS button can be bound as well, and the on-screen Talk button does the same thing. A second key, and a second stick button, can be set to always run Commands even when the header is on Chat.

Chat and Commands each have a gesture. Chat and Mixed start on **Press twice**. Commands starts on **Hold to talk**, so a short order can go out on the release.

**Press twice**

1. **Press once and speak.** Releasing that press keeps listening. A short tick means the next press will send.
2. **Press again to send.**
3. **Hold that second press** (about 700 ms) **to cancel** before the line is transcribed. That is the way out of a stumble.

**Hold to talk**

1. **Hold and speak.**
2. **Release to send.**

A recording longer than 30 seconds sends itself. The Cancel button on the desk still drops a line that is already being transcribed.

Command macros wait between keys (100 ms unless you change it) so a sequence such as Tab, 0, 8 can open a radio menu. Setup, next to each button, chooses the gesture. Next to the commands button, one slider sets the gap between keys and another sets a wait before the first key. A sequence that already contains `delay:120` keeps that wait.

While you are typing in Callsign's own fields, the keyboard talk button is released so the key can be typed.

## What a line does

The header switches three modes:

- **Chat** types the transcript into the active channel. All pilots opens with Enter. Friendly chat opens with Right Ctrl+Enter. The line is cleaned up (control characters, Whisper's silence tags, a 160 character cap) and then typed.
- **Commands** matches the words, in order, to a phrase on the Commands page and presses that key sequence. "Gear up" presses G. "Request landing" can press Tab, then 0, then 8, with a short wait between them. A phrase with no keys is reported and nothing is typed. Short words such as "up" and "down" have to be exact. Longer words allow a small mishear.
- **Mixed** treats a line as a command only when it is almost nothing but the command. A sentence goes to chat. An unbound command is reported rather than typed into chat.

Weapons start disabled, and eject and the gun trigger are not in the list. A misheard word should not drop ordnance. The printed Great Battles defaults are a starting point. Korea uses that map unless you have changed it. Open Controls in the sim and check the keys before you enable key output.

## Run it

You need a recent stable Rust (1.85 or newer).

Linux packages:

```bash
sudo apt install pkg-config cmake g++ libclang-dev libasound2-dev libsdl2-dev
```

SDL is bundled with the build, so the system SDL headers are optional. A C++ compiler is required for the bundled SDL and for the local speech model. On Linux, global hotkeys use X11.

Windows, in PowerShell, from a copy of the folder on the Windows drive (a build made inside WSL cannot type into IL-2):

1. Rust from [rustup.rs](https://rustup.rs), the default MSVC toolchain.
2. Visual Studio Build Tools, with the “Desktop development with C++” workload. The generator `Visual Studio 17 2022` in a failed build means this part is already present.
3. CMake from [cmake.org/download](https://cmake.org/download/). In the installer, choose **Add CMake to the system PATH**. Close PowerShell and open it again, then `cmake --version` must print a version. `sdl2-sys` fails with `is cmake not installed?` when this step was skipped. Visual Studio’s own CMake does not count unless that `bin` folder is on PATH. CMake 4 also needs `CMAKE_POLICY_VERSION_MINIMUM=3.5`, which this repo sets in `.cargo/config.toml`.
4. LLVM, which supplies `libclang`. From PowerShell: `winget install LLVM.LLVM`. The installer does not put `C:\Program Files\LLVM\bin` on PATH. Add that folder to your user Path, set `LIBCLANG_PATH` to the same folder, then open a new PowerShell and check `clang --version`. The bundled SDL library and the speech model both compile from C or C++.

```powershell
cd C:\src\Callsign
cargo build --release
```

The program is one file, `target\release\callsign.exe`. SDL is linked into that file, so the build does not leave an `SDL2.dll` beside it. The speech engine is linked in the same way. The keycap in `assets/icon.png` is embedded in the executable, and the same image is the window and taskbar icon. The model file Setup downloads is still separate, and so are Windows' own libraries. On Linux, from the project folder, `cargo run --release` does the same job.

The first release build compiles the local speech engine and takes a while. To skip that and use only a speech server:

```bash
cargo run --release --no-default-features
```

If `cc` on your machine is Clang and the link step cannot find `libstdc++`, link with GCC instead: `RUSTFLAGS="-C linker=gcc" cargo run --release`.

If the build cannot find `libclang`, point it at the LLVM library directory, for example:

```bash
LIBCLANG_PATH=/usr/lib/llvm-18/lib cargo run --release
```

On Linux a HOTAS may stay invisible until your user is in the `input` group. Pick a stick button that is not also bound inside IL-2. Callsign sees it as a button index.

## Speech

Setup can download the small English Whisper model (or the multilingual one) the first time you want to speak. The file is stored in the app data folder and is not part of this repository. Typed lines on the desk work before the model exists.

Alternatively, point Setup at any server that accepts `POST /audio/transcriptions` the way the OpenAI audio API does, and add a key. The key stays in your config file and is not written to the log.

To try a recording from the command line, using whatever engine Setup is set to:

```bash
cargo run --release -- --transcribe sample.wav
```

The file must be 16-bit PCM WAV.

## Where settings live

Callsign writes its own config the first time you change something. On Linux that is under the user config directory (`callsign/config.json`, readable only by you). Commands can be exported and imported from the Commands page. That page groups them by category and has a search box for the name, a phrase, or the keys. A log of what was heard and sent is written next to the config.

## Assumptions

- Callsign sends keystrokes into a window whose title contains "IL-2". It checks the focused window before it types, and it will not type if that check fails.
- Left and right modifiers are written out (`lctrl`, `rctrl`, `lalt`, `ralt`). A bare `ctrl` is the left key, which is the wrong key for friendly chat.
- Wheel brakes are omitted. They are a hold, not a tap. Parking brake is Left Shift+Slash.
- Numbered radio slots start empty. Put your own key sequence in before you enable them.
