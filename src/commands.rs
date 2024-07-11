use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

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
        options: ExtractOptions,
    },

    /// Pack a file into a GameCube file format
    #[clap(arg_required_else_help = true)]
    Pack {
        file: PathBuf,

        #[clap(short = 'o', long)]
        out: Option<PathBuf>,

        #[clap(flatten)]
        options: PackOptions,
    },
}

#[derive(Debug, Clone, Copy, Args)]
pub struct ExtractOptions {
    #[clap(long, default_value_t = false, action = ArgAction::Set)]
    pub extract_bti: bool,

    #[clap(long, default_value_t = true, action = ArgAction::Set)]
    pub extract_bmg: bool,

    #[clap(long, default_value_t = false, action = ArgAction::Set)]
    pub szs_preserve_extension: bool,
}

#[derive(Debug, Clone, Args)]
pub struct PackOptions {
    #[clap(long, short = 'd', default_value_t = false)]
    pub delete_originals: bool,

    #[clap(long, default_value_t = true, action = ArgAction::Set)]
    pub arc_yaz0_compress: bool,

    #[clap(long)]
    pub arc_extension: Option<String>,
}

impl PackOptions {
    pub fn arc_extension(&self) -> &str {
        self.arc_extension
            .as_deref()
            .unwrap_or_else(|| if self.arc_yaz0_compress { "szs" } else { "arc" })
    }
}
