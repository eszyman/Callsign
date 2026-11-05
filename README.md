# Callsign

Callsign is a desktop companion for **IL-2 Sturmovik** (Great Battles, including Korea). It sits beside the simulator. Press a button, speak, then press again to send. It can type a chat line, or turn a short phrase into the keys the sim already uses for the radio, gear, flaps, and the rest of the cockpit.

It is not an overlay inside the headset. The microphone is the PC input (a headset mic that the operating system can see works, including some VR mics). Cue tones play on the default output, which SteamVR usually mirrors into the headset.

Key output starts **off**. Until you turn it on in Setup, Callsign only shows what it would have typed.

## The gesture

One push-to-talk control is shared by chat and commands. Scroll Lock is the default, because IL-2 rarely wants that key. A joystick or HOTAS button can be bound as well. The on-screen Talk button does the same thing.

1. **Press once and speak.** A second tap inside the double-tap window (320 ms) cancels an accidental press.
2. After that window, a short tick means the next press will send. **Press once to send.**
3. **Press twice** (a second tap inside the same window) **to cancel** instead.
4. If you do nothing, the arming window expires and the line is sent. A recording longer than 30 seconds sends itself.

While you are typing in Callsign's own fields, the keyboard talk button is released so the key can be typed.

## What a line does

The header switches three modes:

- **Chat** types the transcript into the active channel. All pilots opens with Enter. Friendly chat opens with Right Ctrl+Enter. The line is cleaned up (control characters, Whisper's silence tags, a 160 character cap) and then typed.
- **Commands** matches the words, in order, to a phrase on the Commands page and presses that key sequence. "Gear up" presses G. A phrase with no keys is reported and nothing is typed. Short words such as "up" and "down" have to be exact. Longer words allow a small mishear.
- **Mixed** treats a line as a command only when it is almost nothing but the command. A sentence goes to chat. An unbound command is reported rather than typed into chat.

Weapons start disabled, and eject and the gun trigger are not in the list. A misheard word should not drop ordnance. The printed Great Battles defaults are a starting point. Korea uses that map unless you have changed it. Open Controls in the sim and check the keys before you enable key output.

## Run it

You need a recent stable Rust (1.85 or newer).

Linux packages:

```bash
sudo apt install pkg-config cmake g++ libclang-dev libasound2-dev libsdl2-dev
```

SDL is bundled with the build, so the system SDL headers are optional. A C++ compiler is required for the bundled SDL and for the local speech model. On Linux, global hotkeys use X11.

Windows: Visual Studio Build Tools (MSVC), CMake, and LLVM so `libclang` is on the path. The local speech model and the bundled SDL both compile from C or C++.

```bash
cargo run --release
```

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

Callsign writes its own config the first time you change something. On Linux that is under the user config directory (`callsign/config.json`, readable only by you). Commands can be exported and imported from the Commands page. A log of what was heard and sent is written next to the config.

## Assumptions

- Callsign sends keystrokes into a window whose title contains "IL-2". It checks the focused window before it types, and it will not type if that check fails.
- Left and right modifiers are written out (`lctrl`, `rctrl`, `lalt`, `ralt`). A bare `ctrl` is the left key, which is the wrong key for friendly chat.
- Wheel brakes are omitted. They are a hold, not a tap. Parking brake is Left Shift+Slash.
- Numbered radio slots start empty. Put your own key sequence in before you enable them.
