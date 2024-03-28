#![windows_subsystem = "windows"]

mod assets;
mod camera;
mod engine;
mod graphics;

use std::net::SocketAddr;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    Launch { address: Option<SocketAddr> },
    Update,
}

impl Default for Command {
    fn default() -> Self {
        Self::Launch { address: None }
    }
}

fn main() -> Result<()> {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

        AttachConsole(ATTACH_PARENT_PROCESS).ok();
    }

    tracing_subscriber::fmt::init();

    match Cli::parse().command.unwrap_or_default() {
        Command::Launch { address } => engine::Engine::new(address)?.run()?,
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
