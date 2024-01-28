mod engine;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    Launch,
    Update,
}

fn main() -> Result<()> {
    match Cli::parse().command.unwrap_or(Command::Launch) {
        Command::Launch => engine::Engine::new()?.run()?,
        Command::Update => {
            let status = self_update::backends::github::Update::configure()
                .repo_owner("chaynabors")
                .repo_name(env!("CARGO_PKG_NAME"))
                .bin_name("github")
                .show_download_progress(true)
                .current_version(self_update::cargo_crate_version!())
                .build()?
                .update()?;

            eprintln!("Update status: `{}`!", status.version());
        }
    }

    Ok(())
}
