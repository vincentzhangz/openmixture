# OpenMixture

A VMix-inspired live production switcher UI built with Rust + Iced.

> [!WARNING]
> **Early Development / Not Stable**
> OpenMixture is still in early development and is **not stable yet**.
> Features, behavior, and file formats may change at any time.
> Expect bugs, incomplete features, and possible crashes.
> **Do not use this in production workflows yet.**

## Current Scope

OpenMixture currently focuses on core studio basics:

- Preview / Program layout with 16:9 buses
- Input bin with default black clips
- Drag-and-drop media input (image + video)
- FFmpeg-based video playback
- Video transport controls: restart, play/pause, scrub
- Basic transition flow: CUT / TAKE with duration control

## Status

Implemented now:

- Basic desktop UI shell
- Input selection window
- Image/video clip preview and program routing
- Video playback orientation handling (including mobile/metadata rotation)
- End-of-video behavior freezes on the last frame

Planned later:

- More transition types and polish
- Audio mixer and advanced bus controls
- NDI and additional input types
- Performance optimizations and production hardening

## Tech Stack

- Rust
- Iced (`0.14`)
- `ffmpeg-next` (`8`)
- `glyphon`
- `iced_font_awesome`

## Run

```bash
cargo run
```

## License

Licensed under Apache License 2.0. See [LICENSE](LICENSE).
