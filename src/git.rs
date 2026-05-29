use std::io;
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use tokio::process::Command;

pub async fn clone(url: impl AsRef<str>, depth: usize) -> io::Result<ExitStatus> {
    Command::new("git")
        .arg("clone")
        .arg(url.as_ref())
        .arg("--depth")
        .arg(depth.to_string())
        .status()
        .await
}

pub async fn pull(dir: impl AsRef<Path>) -> io::Result<ExitStatus> {
    Command::new("git")
        .arg("pull")
        .current_dir(dir.as_ref())
        .status()
        .await
}

pub async fn ls_remote(url: impl AsRef<str>, what: impl AsRef<str>) -> io::Result<String> {
    let output = Command::new("git")
        .arg("ls-remote")
        .arg(url.as_ref())
        .arg(what.as_ref())
        .stderr(Stdio::piped())
        .output()
        .await?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\t')
        .nth(0)
        .unwrap_or_default()
        .trim()
        .to_string())
}

pub mod hub {
    use std::process::ExitStatus;

    pub async fn clone(
        owner: impl AsRef<str>,
        repo: impl AsRef<str>,
        depth: usize,
    ) -> std::io::Result<ExitStatus> {
        let url = format!(
            "https://github.com/{}/{}.git",
            owner.as_ref(),
            repo.as_ref()
        );
        super::clone(url, depth).await
    }
}
