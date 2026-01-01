#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
struct IconMeta {
    offset: u32,
    len: u32,
    width: u16,
    height: u16,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut icons_dir: Option<PathBuf> = None;
    let mut out_dir: PathBuf = PathBuf::from(".");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--icons-dir" => {
                i += 1;
                icons_dir = args.get(i).map(PathBuf::from);
            }
            "--out" => {
                i += 1;
                out_dir = args
                    .get(i)
                    .map(PathBuf::from)
                    .expect("--out requires a path");
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            other => {
                eprintln!("Unknown arg: {other}");
                print_help();
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icons_repo_dir = icons_dir.unwrap_or_else(|| find_icons_repo_dir(&manifest_dir));
    let platform_root = find_platform_root(&icons_repo_dir);

    let data_dir = out_dir.join("data");
    let src_dir = out_dir.join("src");
    fs::create_dir_all(&data_dir).expect("create data/");
    fs::create_dir_all(&src_dir).expect("create src/");

    let mut alpha_blob: Vec<u8> = Vec::new();

    // platform -> category -> icon -> meta
    let mut icons_by_category: BTreeMap<String, BTreeMap<String, IconMeta>> = BTreeMap::new();
    let mut first_icon_path_for_name: BTreeMap<String, String> = BTreeMap::new();

    let platform_name = platform_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("android")
        .to_string();

    for category_entry in fs::read_dir(&platform_root)
        .unwrap_or_else(|e| panic!("Failed to read icon categories under {:?}: {e}", platform_root))
    {
        let category_entry = category_entry.expect("Failed to read directory entry");
        let category_path = category_entry.path();
        if !category_path.is_dir() {
            continue;
        }

        let Some(category_name) = category_path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        for icon_entry in fs::read_dir(&category_path)
            .unwrap_or_else(|e| panic!("Failed to read icons under {:?}: {e}", category_path))
        {
            let icon_entry = icon_entry.expect("Failed to read directory entry");
            let icon_path = icon_entry.path();
            if !icon_path.is_dir() {
                continue;
            }

            let Some(icon_name) = icon_path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            if let Some(png_path) = pick_baseline_png_48(&icon_path) {
                let (alpha, width, height) = load_png_alpha(&png_path)
                    .unwrap_or_else(|e| panic!("Failed decoding {:?}: {e}", png_path));

                let offset = alpha_blob.len() as u32;
                alpha_blob.extend_from_slice(&alpha);
                let len = alpha.len() as u32;

                icons_by_category
                    .entry(category_name.to_string())
                    .or_default()
                    .insert(
                        icon_name.to_string(),
                        IconMeta {
                            offset,
                            len,
                            width,
                            height,
                        },
                    );

                first_icon_path_for_name
                    .entry(icon_name.to_string())
                    .or_insert_with(|| format!("{platform_name}/{category_name}/{icon_name}"));
            }
        }
    }

    // Write alpha blob.
    let alpha_bin_path = data_dir.join("material_design_icons_alpha.bin");
    fs::write(&alpha_bin_path, &alpha_blob)
        .unwrap_or_else(|e| panic!("Failed writing {:?}: {e}", alpha_bin_path));

    // Write LZ4 blob (size-prepended).
    let alpha_lz4_path = data_dir.join("material_design_icons_alpha.lz4");
    let compressed = lz4_flex::compress_prepend_size(&alpha_blob);
    fs::write(&alpha_lz4_path, &compressed)
        .unwrap_or_else(|e| panic!("Failed writing {:?}: {e}", alpha_lz4_path));

    // Generate Rust module.
    let rs_path = src_dir.join("material_icons.rs");
    let generated_rs = generate_rust_module(
        &platform_name,
        &icons_by_category,
        &first_icon_path_for_name,
    );
    fs::write(&rs_path, generated_rs).unwrap_or_else(|e| panic!("Failed writing {:?}: {e}", rs_path));

    eprintln!("Wrote: {:?}", alpha_bin_path);
    eprintln!("Wrote: {:?}", alpha_lz4_path);
    eprintln!("Wrote: {:?}", rs_path);
}

fn print_help() {
    eprintln!("gen_icons --icons-dir <path-to-material-design-icons> [--out <repo-root>]");
}

fn find_icons_repo_dir(manifest_dir: &Path) -> PathBuf {
    if let Ok(env) = std::env::var("MATERIAL_DESIGN_ICONS_DIR") {
        let p = PathBuf::from(env);
        if p.is_dir() {
            return p;
        }
        panic!("MATERIAL_DESIGN_ICONS_DIR is set but not a directory: {p:?}");
    }

    for ancestor in manifest_dir.ancestors() {
        let candidate = ancestor.join("material-design-icons");
        if candidate.is_dir() {
            return candidate;
        }
    }

    panic!(
        "Could not locate a 'material-design-icons' directory. Pass --icons-dir or set MATERIAL_DESIGN_ICONS_DIR."
    );
}

fn find_platform_root(repo_dir: &Path) -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(repo_dir)
        .unwrap_or_else(|e| panic!("Failed to read icons repo directory {:?}: {e}", repo_dir))
    {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_dir() && path.join("action").is_dir() {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => candidates.remove(0),
        0 => panic!(
            "Could not find a platform root under {:?} containing an 'action' category folder",
            repo_dir
        ),
        _ => panic!(
            "Multiple platform roots found under {:?} (each contains an 'action' folder): {candidates:?}",
            repo_dir
        ),
    }
}

fn pick_baseline_png_48(icon_dir: &Path) -> Option<PathBuf> {
    let base = icon_dir
        .join("materialicons")
        .join("black")
        .join("res")
        // mdpi is the baseline density; the *_48 assets here are typically 48x48.
        // Higher densities (e.g. xxxhdpi) can be 4x larger in pixels, which bloats the blob.
        .join("drawable-mdpi");

    if !base.is_dir() {
        return None;
    }

    let mut best: Option<PathBuf> = None;
    for entry in fs::read_dir(&base).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        if name.ends_with("_black_48.png") && name.starts_with("baseline_") {
            best = Some(path);
            break;
        }
        if name.ends_with("_black_48.png") {
            best = Some(path);
        }
    }

    best
}

fn load_png_alpha(path: &Path) -> Result<(Vec<u8>, u16, u16), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png read_info: {e}"))?;

    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png next_frame: {e}"))?;

    let width = info.width;
    let height = info.height;
    let data = &buf[..info.buffer_size()];

    let mut alpha = Vec::with_capacity((width * height) as usize);

    match info.color_type {
        png::ColorType::Rgba => {
            for px in data.chunks_exact(4) {
                alpha.push(px[3]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in data.chunks_exact(2) {
                alpha.push(px[1]);
            }
        }
        png::ColorType::Rgb => {
            // No alpha channel => fully opaque.
            alpha.resize((width * height) as usize, 255);
        }
        png::ColorType::Grayscale => {
            alpha.resize((width * height) as usize, 255);
        }
        png::ColorType::Indexed => {
            return Err("indexed PNG decode did not expand as expected".to_string());
        }
    }

    Ok((alpha, width as u16, height as u16))
}

fn sanitize_ident(raw: &str) -> String {
    let mut out = String::new();
    for (i, ch) in raw.chars().enumerate() {
        let valid = ch == '_' || ch.is_ascii_alphanumeric();
        if i == 0 && ch.is_ascii_digit() {
            out.push('_');
        }
        out.push(if valid { ch } else { '_' });
    }

    if out.is_empty() {
        out.push('_');
    }

    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
        "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
        "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await",
        "dyn", "try", "yield",
    ];

    if KEYWORDS.contains(&out.as_str()) {
        out = format!("r#{out}");
    }

    out
}

fn generate_rust_module(
    platform_name: &str,
    icons_by_category: &BTreeMap<String, BTreeMap<String, IconMeta>>,
    first_icon_path_for_name: &BTreeMap<String, String>,
) -> String {
    let mut out = String::new();

    out.push_str("// Generated icon table.\n");
    out.push_str("//\n");
    out.push_str("// This file is generated by src/bin/gen_icons.rs.\n");
    out.push_str("// It embeds baseline 48px icons as ALPHA8 for tintable UI icons.\n\n");

    out.push_str("#![allow(non_upper_case_globals)]\n\n");

    out.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n");
    out.push_str("pub struct IconId {\n");
    out.push_str("    pub offset: u32,\n");
    out.push_str("    pub len: u32,\n");
    out.push_str("    pub width: u16,\n");
    out.push_str("    pub height: u16,\n");
    out.push_str("}\n\n");

    out.push_str("impl IconId {\n");
    out.push_str("    pub fn alpha(self) -> &'static [u8] {\n");
    out.push_str("        let start = self.offset as usize;\n");
    out.push_str("        let end = start + self.len as usize;\n");
    out.push_str("        &alpha_blob()[start..end]\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("#[cfg(feature = \"uncompressed\")]\n");
    out.push_str("static ICON_ALPHA: &[u8] = include_bytes!(\"../data/material_design_icons_alpha.bin\");\n\n");

    out.push_str("#[cfg(all(not(feature = \"uncompressed\"), feature = \"lz4\"))]\n");
    out.push_str("static ICON_ALPHA_LZ4: &[u8] = include_bytes!(\"../data/material_design_icons_alpha.lz4\");\n\n");

    out.push_str("fn alpha_blob() -> &'static [u8] {\n");
    out.push_str("    #[cfg(feature = \"uncompressed\")]\n");
    out.push_str("    {\n");
    out.push_str("        ICON_ALPHA\n");
    out.push_str("    }\n\n");

    out.push_str("    #[cfg(all(not(feature = \"uncompressed\"), feature = \"lz4\"))]\n");
    out.push_str("    {\n");
    out.push_str("        use std::sync::OnceLock;\n");
    out.push_str("        static DECOMPRESSED: OnceLock<Vec<u8>> = OnceLock::new();\n");
    out.push_str("        DECOMPRESSED\n");
    out.push_str("            .get_or_init(|| {\n");
    out.push_str("                lz4_flex::decompress_size_prepended(ICON_ALPHA_LZ4)\n");
    out.push_str("                    .expect(\"failed to decompress embedded icon blob\")\n");
    out.push_str("            })\n");
    out.push_str("            .as_slice()\n");
    out.push_str("    }\n\n");

    out.push_str("    #[cfg(all(not(feature = \"uncompressed\"), not(feature = \"lz4\")))]\n");
    out.push_str("    {\n");
    out.push_str("        compile_error!(\"Enable either feature `lz4` (default) or `uncompressed`.\");\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Platform module.
    let platform_ident = sanitize_ident(platform_name);
    out.push_str(&format!("/// Upstream platform root.\npub mod {platform_ident} {{\n"));

    for (category, icons) in icons_by_category.iter() {
        let cat_ident = sanitize_ident(category);
        out.push_str(&format!("    pub mod {cat_ident} {{\n"));
        for (icon, meta) in icons.iter() {
            let icon_ident = sanitize_ident(icon);
            out.push_str(&format!(
                "        pub const {icon_ident}: super::super::IconId = super::super::IconId {{ offset: {offset}u32, len: {len}u32, width: {w}u16, height: {h}u16 }};\n",
                offset = meta.offset,
                len = meta.len,
                w = meta.width,
                h = meta.height
            ));
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n\n");

    // Compatibility re-exports (category modules at the root, like older generator output).
    out.push_str(&format!("pub use {platform_ident}::*;\n\n"));

    // ALL list.
    out.push_str("/// All embedded icons as (\"platform/category/icon\", IconId).\n");
    out.push_str("pub const ALL: &[(&str, IconId)] = &[\n");
    for (category, icons) in icons_by_category.iter() {
        let cat_ident = sanitize_ident(category);
        for (icon, _) in icons.iter() {
            let icon_ident = sanitize_ident(icon);
            out.push_str(&format!(
                "    (\"{platform}/{category}/{icon}\", {platform_ident}::{cat_ident}::{icon_ident}),\n",
                platform = platform_name,
                category = category,
                icon = icon
            ));
        }
    }
    out.push_str("];\n\n");

    // by_path lookup.
    out.push_str("/// Lookup by full path like \"android/action/account_balance\" (case-insensitive).\n");
    out.push_str("pub fn by_path(path: &str) -> Option<IconId> {\n");
    out.push_str("    let key = path.trim().to_ascii_lowercase();\n");
    out.push_str("    match key.as_str() {\n");
    for (category, icons) in icons_by_category.iter() {
        let cat_ident = sanitize_ident(category);
        for (icon, _) in icons.iter() {
            let icon_ident = sanitize_ident(icon);
            out.push_str(&format!(
                "        \"{platform}/{category}/{icon}\" => Some({platform_ident}::{cat_ident}::{icon_ident}),\n",
                platform = platform_name.to_ascii_lowercase(),
                category = category.to_ascii_lowercase(),
                icon = icon.to_ascii_lowercase(),
            ));
        }
    }
    out.push_str("        _ => None,\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // by_name lookup (first match).
    out.push_str("/// Lookup by icon name like \"account_balance\" (case-insensitive).\n");
    out.push_str("/// If multiple categories contain the same icon name, the first one (stable) wins.\n");
    out.push_str("pub fn by_name(name: &str) -> Option<IconId> {\n");
    out.push_str("    let key = name.trim().to_ascii_lowercase();\n");
    out.push_str("    match key.as_str() {\n");

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (icon_name, path) in first_icon_path_for_name.iter() {
        if !seen.insert(icon_name.to_ascii_lowercase()) {
            continue;
        }
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 3 {
            panic!("bad icon path: {path}");
        }
        let category = parts[1];
        let icon = parts[2];
        let cat_ident = sanitize_ident(category);
        let icon_ident = sanitize_ident(icon);
        out.push_str(&format!(
            "        \"{name}\" => Some({platform_ident}::{cat_ident}::{icon_ident}),\n",
            name = icon_name.to_ascii_lowercase()
        ));
    }

    out.push_str("        _ => None,\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}
