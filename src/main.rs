use workflowo::cli;

fn main() {
    let args = cli::parse_and_validate_args();
    println!("{:?}", args);
}
