use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use tokio::process::Command;

pub const VOID_OWNER: &str = "void-linux";
pub const VOID_REPO: &str = "void-packages";

const ARCH: &str = "x86_64";

pub fn get_root() -> io::Result<PathBuf> {
    let mut path = std::env::current_dir()?;
    path.push(VOID_REPO);
    Ok(path)
}

fn get_masterdir() -> io::Result<PathBuf> {
    let dir = format!("masterdir-{}", ARCH);
    let mut path = get_root()?;
    path.push(dir);
    Ok(path)
}

fn get_hostdir() -> io::Result<PathBuf> {
    let mut path = get_root()?;
    path.push("hostdir");
    Ok(path)
}

pub fn get_binpkgs() -> io::Result<PathBuf> {
    let mut path = get_hostdir()?;
    path.push("binpkgs");
    Ok(path)
}

pub fn get_srcpkgs() -> io::Result<PathBuf> {
    let mut path = get_root()?;
    path.push("srcpkgs");
    Ok(path)
}

fn get_xbps_src() -> io::Result<PathBuf> {
    let mut root = get_root()?;
    root.push("xbps-src");
    std::fs::canonicalize(root)
}

pub async fn bootstrap() -> io::Result<ExitStatus> {
    let mut cmd = Command::new(get_xbps_src()?);

    if get_masterdir()?.exists() {
        cmd.arg("bootstrap-update");
    } else {
        cmd.arg("binary-bootstrap");
    }

    cmd.current_dir(&get_root()?).status().await
}

pub async fn pkg(pkg: impl AsRef<str>) -> io::Result<ExitStatus> {
    let mut cmd = Command::new(get_xbps_src()?);
    cmd.arg("pkg");
    cmd.arg(pkg.as_ref());
    cmd.current_dir(&get_root()?);
    cmd.status().await
}
