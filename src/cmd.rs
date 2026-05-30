use crate::template::Template;
use anyhow::Context;
use bytes::Bytes;
use futures_util::StreamExt;
use octocrab::Octocrab;
use octocrab::params::repos::Reference;
use octocrab::repos::releases::MakeLatest;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::info;
use tracing::log::warn;

fn templates(path: impl AsRef<Path>) -> anyhow::Result<Vec<PathBuf>> {
    Ok(std::fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("template"))
        .filter(|entry| entry.is_file())
        .collect())
}

fn assets(path: impl AsRef<Path>) -> anyhow::Result<Vec<PathBuf>> {
    let mut assets = vec![];

    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();

        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        if name.ends_with(".xbps")
            || name.ends_with(".xbps.sig2")
            || name.ends_with("-repodata")
            || name.ends_with("-repodata.sig2")
        {
            assets.push(path);
        }
    }

    Ok(assets)
}

pub async fn update(srcpkgs: PathBuf, client: &Octocrab) -> anyhow::Result<()> {
    for entry in templates(&srcpkgs)? {
        info!("Checking {}", entry.display());
        let mut template = Template::from_file(entry)?;
        crate::update::update(&mut template, client).await?;
    }

    Ok(())
}

pub fn plan(
    srcpkgs: PathBuf,
    binpkgs: PathBuf,
    output: PathBuf,
    force_rebuild: bool,
) -> anyhow::Result<()> {
    let mut updated = vec![];
    for entry in templates(srcpkgs)? {
        let template = Template::from_file(entry)?;
        let name = template
            .get_single("pkgname")
            .ok_or_else(|| anyhow::anyhow!("pkgname not found"))?;

        if force_rebuild {
            updated.push(name);
            continue;
        }

        let bin_name = template.get_asset_name().ok_or_else(|| {
            anyhow::anyhow!("Failed to determine asset name for package '{}'", name)
        })?;

        if !binpkgs.join(&bin_name).exists() {
            updated.push(name);
        }
    }

    let json = updated.join(" ");
    let mut file = OpenOptions::new().append(true).open(output)?;

    writeln!(file, "packages={}", json)?;

    file.sync_all()?;

    Ok(())
}

pub async fn restore(
    remote: String,
    srcpkgs: PathBuf,
    dest: PathBuf,
    client: &Octocrab,
) -> anyhow::Result<()> {
    let mut desired = HashSet::new();
    for entry in templates(&srcpkgs)? {
        let template = Template::from_file(entry)?;
        let bin_name = template.get_asset_name().ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to determine asset name for package '{}'",
                template.get_single("pkgname").unwrap_or_default()
            )
        })?;
        desired.insert(bin_name);
    }

    let (owner, repo) = remote
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid GitHub repo '{}'", remote))?;
    let release = client.repos(owner, repo).releases().get_latest().await?;
    for asset in release.assets {
        let name = asset.name;
        let url = asset.browser_download_url;

        // download only release assets that haven't had their templates updated.
        if !desired.contains(&name) {
            continue;
        }

        let path = dest.join(name);
        let resp = reqwest::get(url.clone())
            .await
            .with_context(|| format!("Failed to download release asset '{}'", url))?
            .error_for_status()?;

        let mut file = File::create(path).await?;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }
    }

    Ok(())
}

pub async fn publish(remote: String, binpkgs: PathBuf, client: &Octocrab) -> anyhow::Result<()> {
    let (owner, repo_name) = remote
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid GitHub repo '{}'", remote))?;

    let repo = client.repos(owner, repo_name);
    let releases = repo.releases();
    let now = chrono::Utc::now();
    let fmt = now.format("%Y%m%d-%H%M%S");
    let new_release = releases
        .create(&fmt.to_string())
        .draft(true)
        .name(&format!("abyss-{fmt}"))
        .body("Automated release created by dredge.")
        .target_commitish("main")
        .make_latest(MakeLatest::True)
        .send()
        .await?;

    let mut binpkgs = assets(&binpkgs)?;
    binpkgs.sort();

    for entry in &binpkgs {
        let data = tokio::fs::read(&entry).await?;
        let file_name = entry
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Failed to get file name for '{}'", entry.display()))?
            .to_string_lossy()
            .to_string();

        releases
            .upload_asset(new_release.id.0, &file_name, Bytes::from(data))
            .send()
            .await
            .with_context(|| format!("Failed to upload new release asset '{}'", entry.display()))?;
    }

    let new_release = releases.get(new_release.id.0).await?;

    if new_release.assets.len() != binpkgs.len() {
        anyhow::bail!(
            "Expected {} assets to be uploaded, but found {}",
            binpkgs.len(),
            new_release.assets.len()
        );
    }

    releases
        .update(new_release.id.0)
        .draft(false)
        .send()
        .await?;

    let all_releases = releases.list().per_page(100).send().await?.items;
    for release in all_releases {
        if release.id == new_release.id {
            continue;
        }

        releases.delete(release.id.0).await?;
        let git_ref = Reference::Tag(release.tag_name.clone());
        if let Err(e) = repo.delete_ref(&git_ref).await {
            warn!("Failed to delete git tag '{}': {e}", release.tag_name);
        }
    }

    Ok(())
}
