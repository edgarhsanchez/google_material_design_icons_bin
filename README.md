# google-material-design-icons-bin

[![CI](https://github.com/edgarhsanchez/google_material_design_icons_bin/actions/workflows/ci.yml/badge.svg)](https://github.com/edgarhsanchez/google_material_design_icons_bin/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/google-material-design-icons-bin.svg)](https://crates.io/crates/google-material-design-icons-bin)
[![Docs.rs](https://docs.rs/google-material-design-icons-bin/badge.svg)](https://docs.rs/google-material-design-icons-bin)
[![License](https://img.shields.io/crates/l/google-material-design-icons-bin.svg)](THIRD_PARTY_NOTICES.md)

Embedded Google Material Design icons packed into a compact binary for fast lookup.

This crate embeds **48×48 baseline** icons as **ALPHA8** (1 byte/pixel). That keeps the data small and lets consumers tint the icon at render time.

## Install

```toml
[dependencies]
google-material-design-icons-bin = "0.1"
```

## API

The generated module mirrors the upstream `material-design-icons/android/` tree:

```rust
use google_material_design_icons_bin::material_icons;

// Direct access via modules (fast, no strings):
let icon = material_icons::android::action::account_balance;
assert_eq!(icon.width, 48);
assert_eq!(icon.height, 48);

// Raw alpha bytes (len == width * height):
let alpha: &[u8] = icon.alpha();
assert_eq!(alpha.len(), icon.width as usize * icon.height as usize);
```

### Lookup helpers

If you need string-based lookup:

```rust
use google_material_design_icons_bin::material_icons;

let icon = material_icons::by_path("android/action/account_balance").unwrap();
let same = material_icons::by_name("account_balance").unwrap();
assert_eq!(icon, same);
```

You can also iterate everything:

```rust
use google_material_design_icons_bin::material_icons;

for (path, icon) in material_icons::ALL {
	let _ = (path, icon.width, icon.height, icon.alpha());
}
```

## Features

- `lz4` (default): embeds an LZ4-compressed alpha blob and decompresses once lazily on first use.
- `uncompressed`: embeds an uncompressed alpha blob (larger crate; no decompression).

Examples:

```bash
# Default (LZ4)
cargo build

# Force uncompressed
cargo build --no-default-features --features uncompressed
```

## Regenerating the icon table

This repo ships with pre-generated files in `data/` and `src/material_icons.rs`.

To regenerate from a local checkout of `material-design-icons`:

```bash
cargo run --bin gen_icons --release -- --icons-dir C:\\github\\material-design-icons --out .
```

Alternatively you can set `MATERIAL_DESIGN_ICONS_DIR` and omit `--icons-dir`.

## Notes

- The embedded images are alpha-only; your renderer is expected to apply tint/color.
- `by_name()` returns the first match if multiple categories contain the same icon name.

## License

- This repository’s code is MIT licensed (see `LICENSE`).
- Embedded icon data is derived from `material-design-icons` and is Apache-2.0 licensed (see `LICENSE-APACHE` and `THIRD_PARTY_NOTICES.md`).
