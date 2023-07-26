use std::{
    env, fmt,
    fmt::Display,
    process::{self, Command},
};
pub mod ssh;

pub trait Task: Display {
    /// Will be called when the task should be executed.
    fn execute(&self);
}

pub struct Job {
    pub name: String,
    children: Vec<Box<dyn Task>>,
}

impl Job {
    pub fn new(name: String) -> Self {
        Self {
            name,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Task>) {
        self.children.push(child);
    }
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

pub trait ShellCommand {
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

        let output = command
            .arg("-c")
            .arg(&self.args.join(" "))
            .output()
            .unwrap();
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

#[derive(Debug)]
pub enum OS {
    Windows,
    Linux,
}

pub struct OSDependent {
    os: OS,
    children: Vec<Box<dyn Task>>,
}

impl OSDependent {
    pub fn new(os: OS) -> Self {
        Self {
            os,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Task>) {
        self.children.push(child)
    }
}

impl Task for OSDependent {
    fn execute(&self) {
        match self.os {
            OS::Windows => {
                if env::consts::OS != "windows" {
                    return;
                }
            }
            OS::Linux => {
                if env::consts::OS != "linux" {
                    return;
                }
            }
        }

        for child in &self.children {
            child.execute();
        }
    }
}

impl Display for OSDependent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = format!("OSDependent: {{ os \"{:?}\" children {{ ", &self.os);
        for child in &self.children {
            text += &format!("{} ", child);
        }
        text += "} }";

        write!(f, "{}", text)
    }
}

#[derive(Debug)]
pub struct PrintTask {
    prompt: String,
}

impl PrintTask {
    pub fn new(prompt: String) -> Self {
        Self { prompt }
    }
}

impl Task for PrintTask {
    fn execute(&self) {
        println!("{}", self.prompt);
    }
}

impl Display for PrintTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
