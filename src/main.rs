use workflowo::cli;
use workflowo::tasks::Task;
use workflowo::yaml_parser;

fn main() {
    let args = cli::parse_and_validate_args();

    let jobs = yaml_parser::jobs_from_file(args.file);
    for job in &jobs {
        if args.verbose {
            println!("{}", job);
        }
        if job.name == args.job {
            println!("Executing Job {}", job.name);
            if let Err(error) = job.execute() {
                eprintln!("Job failed:\n╔══\n{}", error);
            }
            return;
        }
    }
    eprintln!("Error! Job {} not found.", args.job);
}
