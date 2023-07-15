pub mod cli;
pub mod yaml_parser;

use std::{
    fmt,
    fmt::Display,
    process::{self, Command},
};

pub trait Task: Display {
    /// Will be called when the task should be executed.
    fn execute(&self);
}

pub struct Job {
    pub name: String,
    children: Vec<Box<dyn Task>>,
}

impl Task for Job {
    fn execute(&self) {
        for child in self.children.iter() {
            child.execute();
        }
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = format!("Job: {{ name \"{}\" children {{ ", &self.name);
        for child in &self.children {
            text += &format!("{} ", child);
        }
        text += "} }";

        write!(f, "{}", text)
    }
}

trait ShellCommand {
    fn new(args: Vec<String>, work_dir: Option<String>) -> Self;
}

#[derive(Debug)]
pub struct Bash {
    args: Vec<String>,
    work_dir: Option<String>,
}

impl ShellCommand for Bash {
    fn new(args: Vec<String>, work_dir: Option<String>) -> Self {
        Bash { args, work_dir }
    }
}

impl Task for Bash {
    fn execute(&self) {
        let mut command = Command::new("bash");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = command.arg("-c").args(&self.args).output().unwrap();
        if !output.status.success() {
            println!(
                "Error: {:?} did not success and raised an error!\n{}",
                &self.args,
                String::from_utf8_lossy(&output.stderr)
            );
            process::exit(-1)
        }
    }
}

impl Display for Bash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

#[derive(Debug)]
pub struct Cmd {
    args: Vec<String>,
    work_dir: Option<String>,
}

impl ShellCommand for Cmd {
    fn new(args: Vec<String>, work_dir: Option<String>) -> Self {
        Cmd { args, work_dir }
    }
}

impl Task for Cmd {
    fn execute(&self) {
        let mut command = Command::new("cmd");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = command.arg("/c").args(&self.args).output().unwrap();
        if !output.status.success() {
            println!(
                "Error: {:?} did not success and raised an error!\n{}",
                &self.args,
                String::from_utf8_lossy(&output.stderr)
            );
            process::exit(-1)
        }
    }
}

impl Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
