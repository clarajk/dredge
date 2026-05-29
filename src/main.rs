use crate::cli::{Args, Command};
use clap::Parser;
use octocrab::Octocrab;

mod cli;
mod cmd;
mod git;
mod template;
mod update;
mod xbps;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls crypto provider");

    let now = chrono::Utc::now();
    let fmt = now.format("abyss-%Y%m%d-%H%M%S");
    tracing::info!("Starting Abyss at {}", fmt);

    return Ok(());

    let args = Args::parse();
    let token = std::env::var("GITHUB_TOKEN")?;
    let github = Octocrab::builder().personal_token(token).build()?;

    match args.command {
        Command::Update { srcpkgs } => cmd::update(srcpkgs, &github).await?,
        Command::Plan {
            srcpkgs,
            binpkgs,
            output,
        } => cmd::plan(srcpkgs, binpkgs, output)?,
        Command::Restore {
            remote,
            srcpkgs,
            dest,
        } => cmd::restore(remote, srcpkgs, dest, &github).await?,
        Command::Publish { .. } => {}
    }

    Ok(())
}
