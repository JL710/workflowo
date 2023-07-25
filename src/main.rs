use workflowo::cli;
use workflowo::yaml_parser;
use workflowo::tasks::Task;

fn main() {
    let args = cli::parse_and_validate_args();

    let jobs = yaml_parser::jobs_from_file(args.file);
    for job in &jobs {
        if args.verbose {
            println!("{}", job);
        }
        if job.name == args.job {
            println!("Executing Job {}", job.name);
            job.execute();
            return;
        }
    }
    println!("Error! Job {} not found.", args.job);
}
