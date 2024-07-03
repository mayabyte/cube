use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

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

        /// Directory or filename for the extracted file(s). Exact behavior depends on what's
        /// being extracted - if extracting a single file, this will specify the filename of the
        /// extracted file, and if the extraction results in multiple files, this will be the name
        /// of the directory they're placed inside.
        ///
        /// If multiple input files are provided, this option will always specify a folder into
        /// which they'll be extracted.
        #[clap(short = 'o', long)]
        out: Option<PathBuf>,

        #[clap(flatten)]
        extract_options: ExtractOptions,
    },

    /// Pack a file into a GameCube file format
    #[clap(arg_required_else_help = true)]
    Pack {
        file: PathBuf,

        #[clap(short = 'o', long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, Args)]
pub struct ExtractOptions {
    #[clap(long)]
    pub extract_bti: bool,

    #[clap(long, default_value_t = true)]
    pub extract_bmg: bool,
}
