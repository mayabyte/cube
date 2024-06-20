use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name="cube", author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub subcommand: Commands,

    #[clap(global = true, default_value_t = 0, short = 'v')]
    pub verbosity: u8,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Extract a file based on its file type and metadata
    #[clap(arg_required_else_help = true)]
    Extract {
        files: Vec<PathBuf>,

        #[clap(long)]
        bti: bool,
    },

    /// Pack a file into a GameCube file format
    #[clap(arg_required_else_help = true)]
    Pack { file: PathBuf, out: Option<PathBuf> },
}
