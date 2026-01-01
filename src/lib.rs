#![forbid(unsafe_code)]

//! Embedded Material Design icons packed into a compact binary.
//!
//! This crate ships with a pre-generated compressed blob in `data/`.
//! To regenerate from a local checkout of `material-design-icons`, run:
//! `cargo run --bin gen_icons --release -- --icons-dir <path>`

pub mod material_icons;
