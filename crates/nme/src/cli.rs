//! Argument parsing. Kept separate from `main.rs` and from the per-command
//! logic in `commands/`, so adding a new subcommand later is: add a variant
//! here, add a module under `commands/`, add one match arm in `main.rs`.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "nme", version, about = "Nyanko's Metadata Editor")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print a file's metadata to stdout.
    #[command(alias = "i")]
    Info(InfoArgs),

    /// Set one metadata field on a file.
    #[command(alias = "s")]
    Set(SetArgs),
}

#[derive(Debug, Parser)]
pub struct InfoArgs {
    /// File(s) to read metadata from.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Print machine-readable JSON instead of human-readable text.
    #[arg(short = 'j', long = "json")]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct SetArgs {
    #[arg(required = true)]
    pub metadata_name: String,

    #[arg(required = true)]
    pub metadata_value: String,

    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}