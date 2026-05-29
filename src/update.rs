use crate::git;
use crate::template::Template;
use anyhow::Context;
use futures_util::StreamExt;
use octocrab::Octocrab;
use regex::{Regex, RegexBuilder};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::LazyLock;
use tracing::{error, info};

static SHELL_VAR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new("\\$\\{([a-z0-9_-]+?)\\}")
        .case_insensitive(true)
        .build()
        .expect("invalid shell var regex")
});

pub async fn update(
    template: &mut Template,
    srcpkgs: impl AsRef<Path>,
    client: &Octocrab,
) -> anyhow::Result<()> {
    let name = template
        .get_single("pkgname")
        .ok_or_else(|| anyhow::anyhow!("pkgname not found"))?;

    let Some(update_source) = template.get_single("_abyss_source") else {
        error!("Update source not specified for package '{}'", name);
        return Ok(());
    };

    let Some(repo) = template.get_single("_abyss_repo") else {
        error!("Update repository not specified for package '{}'", name);
        return Ok(());
    };

    let strip = template.get_single("_abyss_strip");

    let (key, new_value) = match &update_source[..] {
        "github-release" => {
            let (owner, repo) = {
                let split: Vec<_> = repo.split('/').collect();
                if split.len() != 2 {
                    error!("Invalid GitHub repo '{}'", repo);
                    return Ok(());
                }

                (split[0], split[1])
            };

            let latest = client
                .repos(owner, repo)
                .releases()
                .get_latest()
                .await?
                .tag_name;
            let latest = if let Some(strip) = strip {
                latest.trim_start_matches(&strip).to_string()
            } else {
                latest
            };

            ("version", latest)
        }
        "git-head" => {
            let head = git::ls_remote(repo, "HEAD").await?;
            ("_commit", head)
        }
        _ => {
            error!(
                "Unknown update source '{}' for package '{}'",
                update_source, name
            );
            return Ok(());
        }
    };

    let old_value = template
        .get_single(&key)
        .ok_or_else(|| anyhow::anyhow!("{} not found in template", key))?;

    if old_value != new_value {
        info!(
            "Updating package '{}' from '{}' to '{}'",
            name, old_value, new_value
        );
        template.set(&key, new_value);
        template.set("revision", "1");
        update_checksum(template).await?;
    }

    Ok(())
}

async fn update_checksum(template: &mut Template) -> anyhow::Result<()> {
    let Some(distfiles) = template.get_single("distfiles") else {
        anyhow::bail!("distfiles not found in template");
    };

    let mut url = distfiles;

    let vars = SHELL_VAR_REGEX
        .captures_iter(&url)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_owned()))
        .collect::<Vec<_>>();

    for var in vars {
        let shell_var = format!("${{{}}}", var.as_str());
        let Some(value) = template.get_single(&var) else {
            anyhow::bail!("shell var '{}' not found in template", var);
        };

        url = url.replace(&shell_var, &value);
    }

    let resp = reqwest::get(&url)
        .await
        .with_context(|| format!("request failed: {url}"))?
        .error_for_status()
        .with_context(|| format!("bad HTTP status: {url}"))?;

    let mut stream = resp.bytes_stream();
    let mut hasher = Sha256::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("error reading response body: {url}"))?;
        hasher.update(&chunk);
    }

    let digest = hasher.finalize();
    let checksum = hex::encode(digest);
    let name = template
        .get_single("pkgname")
        .ok_or_else(|| anyhow::anyhow!("pkgname not found"))?;
    let old_checksum = template
        .get_single("checksum")
        .ok_or_else(|| anyhow::anyhow!("checksum not found"))?;

    info!(
        "Updating '{}' checksum from '{}' to '{}'",
        name, old_checksum, checksum
    );

    template.set("checksum", checksum);

    Ok(())
}
