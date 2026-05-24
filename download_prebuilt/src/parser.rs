use regex::Regex;
use std::{collections::HashMap, path::Path};

use crate::download_apple::{BASE_URL, BinaryPayload};

pub async fn load_version(a: u8, b: u8, c: u8) -> anyhow::Result<Vec<BinaryPayload>> {
    let version = format!("{a}.{b}.{c}");

    let dl_url = format!(
        "https://raw.githubusercontent.com/pytorch/executorch/refs/heads/swiftpm-{version}/Package.swift"
    );

    let buf = reqwest::get(dl_url)
        .await?
        .error_for_status()?
        .text()
        .await?;
    //println!("{buf}");
    let manifest = parse_swift_manifest(&buf)?;
    println!("{manifest:#?}");
    Ok(manifest
        .iter()
        .map(|(k, v)| BinaryPayload {
            name: format!("{}-{}.zip", k, version),
            url: format!("{}{}-{}.zip", BASE_URL, k, version),
            expected_sha: v.to_string(),
        })
        .collect())
}
pub fn parse_spm(path: impl AsRef<Path>) -> anyhow::Result<HashMap<String, String>> {
    let buffer = std::fs::read_to_string(path)?;
    parse_swift_manifest(&buffer)
}
fn parse_swift_manifest(manifest_content: &str) -> anyhow::Result<HashMap<String, String>> {
    let mut checksums = HashMap::new();

    // This regex looks for block patterns like: "backend_coreml": [ "sha256": "..." ]
    // or "sha256" + debug_suffix: "..."
    let block_rx = Regex::new(r#""(?P<name>[a-zA-Z0-9_]+)"\s*:\s*\[([^\]]+)\]"#)?;
    let sha_rx =
        Regex::new(r#""sha256"(?P<debug>\s*\+\s*debug_suffix)?\s*:\s*"(?P<sha>[a-fA-F0-9]{64})""#)?;

    for block_cap in block_rx.captures_iter(manifest_content) {
        let name = &block_cap["name"];
        let block_body = &block_cap[2];

        for sha_cap in sha_rx.captures_iter(block_body) {
            let sha = sha_cap["sha"].to_string();

            // Determine if this specific line inside the dictionary belongs to a debug configuration
            if sha_cap.name("debug").is_some() {
                checksums.insert(format!("{}_debug", name), sha);
            } else {
                checksums.insert(name.to_string(), sha);
            }
        }
    }

    Ok(checksums)
}
