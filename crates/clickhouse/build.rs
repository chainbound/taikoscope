use std::{env, fs, path::PathBuf};

fn main() {
    // Locate dashboard/sequencerConfig.ts from the workspace root
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let seq_cfg = manifest_dir.join("..").join("..").join("dashboard").join("sequencerConfig.ts");

    // If the dashboard file is missing (e.g., minimal builds), just generate empty arrays
    let content = fs::read_to_string(&seq_cfg).unwrap_or_else(|_| String::new());

    // Very simple parser to extract pairs like ['0xabc...', 'Name'] from SEQUENCER_PAIRS
    let mut addrs: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();

    let mut in_pairs = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if !in_pairs {
            if trimmed.starts_with("export const SEQUENCER_PAIRS") && trimmed.contains('[') {
                in_pairs = true;
            }
            continue;
        }

        if trimmed.starts_with("];") {
            break;
        }

        // Skip comments or empty lines
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Expect lines like: ['0x..', 'Name'],
        // Be tolerant to optional trailing commas and spaces
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_str = false;
        let mut quote = '\'';
        for ch in trimmed.chars() {
            if in_str {
                if ch == quote {
                    in_str = false;
                    parts.push(current.clone());
                    current.clear();
                } else {
                    current.push(ch);
                }
            } else if ch == '\'' || ch == '"' {
                in_str = true;
                quote = ch;
            }
        }

        if parts.len() == 2 {
            let addr = parts[0].trim().to_lowercase();
            let name = parts[1].trim().to_string();
            if addr.starts_with("0x") && addr.len() == 42 {
                addrs.push(addr);
                names.push(name);
            }
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest = out_dir.join("sequencer_mapping.rs");

    let addr_entries = addrs.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(", ");
    let name_entries = names
        .iter()
        .map(|n| n.replace('\'', "\\'"))
        .map(|n| format!("\"{}\"", n))
        .collect::<Vec<_>>()
        .join(", ");

    let generated = format!(
        "pub const SEQUENCER_ADDRS: &[&str] = &[{}];\n\
         pub const SEQUENCER_NAMES: &[&str] = &[{}];\n",
        addr_entries, name_entries
    );

    fs::write(&dest, generated).expect("failed to write sequencer_mapping.rs");

    // Rebuild if the TS file changes
    println!("cargo:rerun-if-changed={}", seq_cfg.display());
}
