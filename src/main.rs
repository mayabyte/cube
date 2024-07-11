mod commands;
mod extract;
mod pack;

use clap::Parser;
use commands::{Cli, Commands};
use extract::try_extract;
use log::LevelFilter;
use pack::try_pack;
use simple_logger::SimpleLogger;
use std::error::Error;

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    init_logger(args.verbosity);

    match args.subcommand {
        Commands::Extract { files, out, options } => try_extract(files, out.as_deref(), options)?,
        Commands::Pack { file, mut out, options } => {
            if out.is_none() && file.is_dir() {
                out = Some(file.with_extension(options.arc_extension()));
            }
            try_pack(file, out.as_deref(), &options)?
        }
    }

    Ok(())
}

fn init_logger(level: u8) {
    let log_level = match level {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };
    SimpleLogger::new()
        .with_level(log_level)
        .init()
        .expect("Failed to initialize logger");
}
