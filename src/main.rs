use workflowo::cli;
use workflowo::yaml_parser;

fn main() {
    let args = cli::parse_and_validate_args();

    let jobs = yaml_parser::jobs_from_file(args.file);
    for job in jobs {
        println!("{}", job);
    }
}
