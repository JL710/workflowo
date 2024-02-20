use workflowo::cli;
use workflowo::tasks::Task;
use workflowo::yaml_parser;

pub fn error_chain_string(error: anyhow::Error) -> String {
    let mut message = String::new();

    let mut chain_iter = error.chain();

    message += &format!("Error: {}\n", chain_iter.next().unwrap());

    for err in chain_iter {
        message += &format!("\nCaused by:\n\t{}", err);
    }

    message
}

fn main() {
    let args = cli::parse_and_validate_args();

    let jobs = match yaml_parser::jobs_from_file(args.file) {
        Ok(x) => x,
        Err(err) => {
            println!("{}", error_chain_string(err));
            std::process::exit(1);
        }
    };
    for job in &jobs {
        if args.verbose {
            println!("{}", job);
        }
        if job.name == args.job {
            println!("Executing Job {}", job.name);
            if let Err(error) = job.execute() {
                println!("{}", error_chain_string(error));
                std::process::exit(1);
            }
            return;
        }
    }
    eprintln!("Error! Job {} not found.", args.job);
}
