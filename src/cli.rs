use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Update {
        #[clap(long)]
        srcpkgs: PathBuf,
    },
    Plan {
        #[clap(long)]
        srcpkgs: PathBuf,

        #[clap(long)]
        binpkgs: PathBuf,

        #[clap(long)]
        output: PathBuf,
    },
    Restore {
        #[clap(long)]
        remote: String,

        #[clap(long)]
        srcpkgs: PathBuf,

        #[clap(long)]
        dest: PathBuf,
    },
    Publish {
        #[clap(long)]
        remote: String,

        #[clap(long)]
        source: PathBuf,
    },
}
