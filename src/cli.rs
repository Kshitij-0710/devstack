use crate::generator;
use crate::project::Stack;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "devstack",
    version,
    about = "One-command local dev environment generator"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(value_enum)]
        stack: Stack,
        name: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            stack,
            name,
            dir,
            force,
        } => generator::init(stack, name, dir, force),
    }
}
