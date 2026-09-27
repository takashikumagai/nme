# nme — Nyanko's Metadata Editor

A GUI/CLI file metadata editor written in Rust.

`nme` reads and edits metadata across multiple file formats through a
single, format-agnostic model, backed by per-format parsing crates.

## Supported formats

- PDF (via `lopdf`)
- MP3 / ID3 tags (via `id3`)
- FLAC (via `metaflac`)
- MP4/M4A (via `mp4ameta`)

## Project layout

This is a Cargo workspace with two crates:

- [`crates/nme-core`](crates/nme-core) — format-agnostic metadata model and
  per-format backends.
- [`crates/nme`](crates/nme) — the `nme` binary (CLI, with a GUI planned).

## Building

```sh
cargo build --release
```

The resulting binary will be at `target/release/nme`.

## Usage

Print metadata for a file:

```sh
nme info <path/to/file>
```

(Available subcommands may grow over time — run `nme --help` for the
current list.)

## Status

Early-stage / work in progress. The `nme-core` library and the CLI `info`
subcommand are functional; more subcommands and a GUI are planned.

## License

MIT
