use crate::cli::{Args, Command};
use clap::Parser;
use octocrab::Octocrab;

mod cli;
mod cmd;
mod template;
mod update;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls crypto provider");

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
        Command::Publish { remote, source } => cmd::publish(remote, source, &github).await?,
    }

    Ok(())
}
