use download_prebuilt::download_android::download_android;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    download_android(1, 2, 0, ".").await?;
    Ok(())
}
