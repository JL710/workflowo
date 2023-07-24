use clap::{self, Parser};
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
pub struct Args {
    /// the configuration file
    pub file: PathBuf,

    /// the job that should be executed
    pub job: String,

    #[arg(short, long)]
    pub verbose: bool,
}

/// Parses the cli arguments given to the program and validates them.
///
/// Validates:
/// - the file exists
/// - the file is a file
/// - the file has the extension yml or yaml
pub fn parse_and_validate_args() -> Args {
    let args = Args::parse();

    if !args.file.exists() {
        println!("Error: {} does not exist!", args.file.to_str().unwrap());
        process::exit(-1);
    } else if !args.file.is_file() {
        println!("Error: {} is not a file!", args.file.to_str().unwrap());
        process::exit(-1);
    } else if args.file.extension().unwrap() != "yml" && args.file.extension().unwrap() != "yaml" {
        println!("Error: {} is not a yaml file!", args.file.to_str().unwrap());
        process::exit(-1);
    }

    args
}
