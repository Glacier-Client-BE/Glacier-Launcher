//! Minimal internal download helper. `gl-java` is meant to be self-contained
//! and testable before `gl-download` is ready; every call site below is
//! marked with a `// TODO: replace with gl_download once available` comment.

use crate::error::{io_err, JavaError, Result};
use futures_util::StreamExt;
use std::path::Path;

/// Download `url` to `dest`, creating parent directories as needed. Writes to
/// a `.part` sibling file first and renames it into place on success so a
/// partial/interrupted download never masquerades as a complete file.
pub async fn download_to_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(io_err(parent))?;
    }

    let resp = client.get(url).send().await?;
    let resp = resp.error_for_status().map_err(JavaError::Http)?;

    let tmp = dest.with_extension(match dest.extension() {
        Some(ext) => format!("{}.part", ext.to_string_lossy()),
        None => "part".to_string(),
    });

    let file = tokio::fs::File::create(&tmp).await.map_err(io_err(&tmp))?;
    // tokio::fs::File dispatches every write to the blocking pool; buffering
    // coalesces the many small network chunks into fewer blocking writes.
    let mut file = tokio::io::BufWriter::with_capacity(256 * 1024, file);
    let mut stream = resp.bytes_stream();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await.map_err(io_err(&tmp))?;
    }
    file.flush().await.map_err(io_err(&tmp))?;
    drop(file);

    tokio::fs::rename(&tmp, dest).await.map_err(io_err(dest))?;
    Ok(())
}

/// GET `url` and return the body as a `String`.
pub async fn get_string(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await?;
    let resp = resp.error_for_status().map_err(JavaError::Http)?;
    Ok(resp.text().await?)
}
