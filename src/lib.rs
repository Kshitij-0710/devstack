pub mod cli;
pub mod generator;
pub mod project;

use anyhow::Result;

pub fn run() -> Result<()> {
    cli::run()
}
