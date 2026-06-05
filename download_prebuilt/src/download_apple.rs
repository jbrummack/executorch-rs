use anyhow::{Context, bail};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::parser::load_version;

pub const BASE_URL: &str = "https://ossci-ios.s3.amazonaws.com/executorch/";
//const VERSION: &str = "1.2.0";
//const BINPATH: &str = "./executorch_binaries";
#[derive(Debug)]
pub struct BinaryPayload {
    pub name: String,
    pub url: String,
    pub expected_sha: String,
}
/*fn create_output_dir() -> anyhow::Result<()> {
    for arch in ARCHITECTURES.iter() {
        std::fs::create_dir_all(format!("./output/{arch}"))?;
    }

    Ok(())
}*/
const ARCHITECTURES: LazyLock<HashSet<&str>> =
    LazyLock::new(|| ["ios-arm64-simulator", "macos-arm64", "ios-arm64"].into());
fn inspect_zip_contents(
    zip_path: impl AsRef<Path>,
    output_path: &str,
    debug_build: bool,
) -> anyhow::Result<()> {
    use std::fs::File;

    // Open the file synchronously
    let file = File::open(zip_path.as_ref())?;

    // Initialize the zip archive reader
    let mut archive = zip::ZipArchive::new(file)?;

    //println!("\n--- Contents of {:?} ---", zip_path.as_ref());
    /*println!(
        "{:<50} {:<15} {:<15}",
        "File Path", "Compressed", "Uncompressed"
    );
    println!("{}", "-".repeat(84));*/

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        // Enforce safe file name extraction
        /*let enclosed_name = file
        .enclosed_name()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Invalid/Unsafe Path".to_string());*/
        let mut architecture = None;
        if let Some(path) = file.enclosed_name() {
            for comp in path.components() {
                let compstr: &str = &comp.as_os_str().to_string_lossy();
                if ARCHITECTURES.contains(compstr) {
                    architecture = Some(compstr.to_string());
                }
            }
        }
        // Skip directories if you only want to see files, or print them explicitly
        if file.is_dir() {
            let mut should_extract = false;
            let mut extract_path = Vec::new();
            if let Some(path) = file.enclosed_name() {
                if let Some(arch) = &architecture {
                    for comp in path.components() {
                        let compstr: &str = &comp.as_os_str().to_string_lossy();
                        if compstr == "Headers" {
                            should_extract = true;
                            //extract_path.push(compstr.to_string());
                            /*let dest_path = format!("./output/{arch}/");
                            println!("{path:?}")*/
                        }
                        if should_extract {
                            extract_path.push(compstr.to_string());
                        }
                    }
                    std::fs::create_dir_all(format!(
                        "{output_path}/{arch}/{}",
                        extract_path.join("/")
                    ))?;
                    //;
                }
            }
            /*if enclosed_name.contains("Headers") && !enclosed_name.contains("debug") {
                println!("{enclosed_name}");
            }*/
            //println!("{:<50} [DIR]", enclosed_name);
        } else {
            let mut should_extract = false;
            let mut extract_path = Vec::new();
            if let Some(path) = file.enclosed_name() {
                //let mut architecture = None;

                for comp in path.components() {
                    let compstr: &str = &comp.as_os_str().to_string_lossy();
                    /*let compstr: &str = &comp.as_os_str().to_string_lossy();
                    if ARCHITECTURES.contains(compstr) {
                        architecture = Some(compstr.to_string());
                    }*/
                    if compstr == "Headers" {
                        should_extract = true;
                        //extract_path.push(compstr.to_string());
                        /*let dest_path = format!("./output/{arch}/");
                        println!("{path:?}")*/
                    }
                    if should_extract {
                        extract_path.push(compstr.to_string());
                    }
                    if compstr.contains(".a") && (compstr.contains("debug") == debug_build) {
                        if let Some(arch) = &architecture {
                            let file_name = compstr
                                .replace("_debug", "")
                                .replace("_simulator", "")
                                .replace("_ios", "")
                                .replace("_macos", "");
                            //let output_dir = PathBuf::from(arch);
                            //let dest_path = output_dir.join(file_name);
                            let dest_path = format!("{output_path}/{arch}/{file_name}");
                            // 4. Stream the data from the archive to your local disk
                            let mut output_file = File::create(&dest_path)?;
                            std::io::copy(&mut file, &mut output_file)?;
                            println!("{arch}/{compstr}");
                        }
                    }
                }
                let p = path.to_string_lossy();
                if p.contains(".c") || p.contains(".h") || p.contains(".cpp") || p.contains(".hpp")
                {
                    if let Some(arch) = &architecture {
                        let outpath = format!("{output_path}/{arch}/{}", extract_path.join("/"));
                        let mut output_file =
                            File::create(&outpath).context(format!("{outpath:?}"))?;
                        std::io::copy(&mut file, &mut output_file)?;
                    }
                }
            }
            /*println!(
                "{:<50} {:<15} {:<15}",
                enclosed_name,
                format!("{} bytes", file.compressed_size()),
                format!("{} bytes", file.size())
            );*/
        }
    }

    Ok(())
}
pub fn blocking_download_version(a: u8, b: u8, c: u8, debug_build: bool) -> anyhow::Result<String> {
    let out = tokio::runtime::Runtime::new()?.block_on(download_version(a, b, c, debug_build))?;
    Ok(out)
}
pub fn blocking_download_version_into(
    a: u8,
    b: u8,
    c: u8,
    path: &str,
    debug_build: bool,
) -> anyhow::Result<String> {
    let out = tokio::runtime::Runtime::new()?.block_on(download_version_into(
        a,
        b,
        c,
        path,
        debug_build,
    ))?;
    Ok(out)
}
pub async fn download_version_into(
    a: u8,
    b: u8,
    c: u8,
    pb: &str,
    debug_build: bool,
) -> anyhow::Result<String> {
    let out_dir = format!("{pb}/executorch_binaries/xt_{a}_{b}_{c}");
    if std::fs::exists(&out_dir)? {
        return Ok(out_dir);
    }
    for arch in ARCHITECTURES.iter() {
        std::fs::create_dir_all(format!("{out_dir}/{arch}"))?;
    }
    let bom = load_version(a, b, c).await?;

    download_binary_payloads(bom, &out_dir).await?;
    for zip in std::fs::read_dir(&out_dir)? {
        let zip = zip?;
        if zip.file_type()?.is_file() {
            inspect_zip_contents(zip.path(), &out_dir, debug_build)?;
            std::fs::remove_file(zip.path())?;
        }
    }
    Ok(out_dir)
}
pub async fn download_version(a: u8, b: u8, c: u8, debug_build: bool) -> anyhow::Result<String> {
    let out_dir_env = env::var("OUT_DIR").context("OUT_DIR")?;
    download_version_into(a, b, c, &out_dir_env, debug_build).await
}
async fn download_binary_payloads(
    downloads: Vec<BinaryPayload>,
    out_dir: impl AsRef<Path>,
) -> anyhow::Result<()> {
    // 2. Generate the flat download list (Release + Debug variants)

    // Ensure our local download folder exists
    let output_dir = out_dir.as_ref();
    //let output_dir = Path::new(BINPATH);
    tokio::fs::create_dir_all(output_dir).await?;

    // 3. Share a single HTTP client across concurrent download tasks
    let client = Arc::new(reqwest::Client::new());
    let mut tasks = Vec::new();

    println!("Starting download of {} binaries...", downloads.len());

    for item in downloads {
        let client_clone = Arc::clone(&client);
        let dest_path = output_dir.join(&item.name);

        let task = tokio::spawn(async move {
            match download_and_verify(&client_clone, &item.url, &dest_path, &item.expected_sha)
                .await
            {
                Ok(_) => println!("Successfully verified and saved: {}", item.name),
                Err(e) => eprintln!("Failed downloading {}: {}", item.name, e),
            }
        });
        tasks.push(task);
    }

    // Wait for all downloads to finish
    futures_util::future::join_all(tasks).await;
    println!("Done!");

    Ok(())
}

pub async fn download_and_verify(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha: &str,
) -> anyhow::Result<()> {
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        bail!("Server returned status: {}", response.status());
    }

    // Streaming response while computing the SHA256 block-by-block
    let mut stream = response.bytes_stream();
    let mut file = File::create(dest).await?;
    let mut hasher = Sha256::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        // Update hash
        hasher.update(&chunk);
        // Write to file
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    // Finalize the hash and compare
    let actual_sha = hex::encode(hasher.finalize());
    if actual_sha != expected_sha {
        // Clean up the invalid file if checksum fails
        let _ = tokio::fs::remove_file(dest).await;
        bail!(
            "Checksum mismatch!\nExpected: {}\nActual:   {}",
            expected_sha,
            actual_sha
        );
    }

    Ok(())
}
