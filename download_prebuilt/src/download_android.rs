use anyhow::Context;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

use crate::download_apple::download_and_verify;
pub fn blocking_download_android(a: u8, b: u8, c: u8) -> anyhow::Result<PathBuf> {
    let out_dir_env = std::env::var("OUT_DIR").context("OUT_DIR")?;
    let pb = PathBuf::from(out_dir_env)
        .join("executorch_binaries")
        .join(format!("android_{a}_{b}_{c}"));
    if let Ok(true) = std::fs::exists(&pb) {
        return Ok(pb);
    }
    let out = pb.clone();
    tokio::runtime::Runtime::new()?.block_on(download_android(a, b, c, pb))?;
    Ok(out)
}
pub async fn download_android(a: u8, b: u8, c: u8, dest: impl AsRef<Path>) -> anyhow::Result<()> {
    let dest = &dest.as_ref();
    let version = format!("{a}.{b}.{c}");
    let url = format!(
        "https://repo1.maven.org/maven2/org/pytorch/executorch-android/{version}/executorch-android-{version}.aar"
    );
    let checksum = format!("{url}.sha256");

    let client = reqwest::Client::new();
    let expected_sha = client
        .get(checksum)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let aar = dest.join("aar");
    download_and_verify(&client, &url, &aar, &expected_sha).await?;
    use std::fs::File;

    // Open the file synchronously
    let file = File::open(&aar)?;

    // Initialize the zip archive reader
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() {
            if let Some(name) = file.enclosed_name() {
                let fname = name.to_string_lossy().to_string();
                if fname.contains("libexecutorch.so") {
                    fn extract(
                        fname: &str,
                        arch: &str,
                        dest: &Path,
                        input: &mut impl Read,
                    ) -> anyhow::Result<()> {
                        if fname.contains(arch) {
                            let path = dest.join(arch);
                            std::fs::create_dir_all(&path)?;
                            let mut output_file = File::create(path.join("libexecutorch.so"))?;
                            std::io::copy(input, &mut output_file)?;
                        }
                        Ok(())
                    }
                    extract(&fname, "x86_64", dest, &mut file)?;
                    extract(&fname, "arm64", dest, &mut file)?;
                }
            }
        }
    }
    Ok(())
}
