mod cli;
mod commands;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Info(args) => commands::info::run(args),
        Command::Set(args) => commands::set::run(args),
    }
}
