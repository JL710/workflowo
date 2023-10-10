use std::{env, fmt, fmt::Display};
mod error;
pub mod shell;
pub mod ssh;
use error::{
    task_dynerror_panic, task_might_panic, task_panic, task_taskerror_panic, SourceError, TaskError,
};

pub trait Task: Display {
    /// Will be called when the task should be executed.
    fn execute(&self) -> Result<(), TaskError>;
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
    fn execute(&self) -> Result<(), TaskError> {
        for (index, child) in self.children.iter().enumerate() {
            if let Err(error) = child.execute() {
                task_taskerror_panic!(
                    format!("Child {}(first is 0) of task {} failed", index, &self.name),
                    error
                );
            }
        }
        Ok(())
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
    fn execute(&self) -> Result<(), TaskError> {
        match self.os {
            OS::Windows => {
                if env::consts::OS != "windows" {
                    // return if not target os
                    return Ok(());
                }
            }
            OS::Linux => {
                if env::consts::OS != "linux" {
                    // return if not target os
                    return Ok(());
                }
            }
        }

        for (index, child) in self.children.iter().enumerate() {
            if let Err(error) = child.execute() {
                task_taskerror_panic!(
                    format!(
                        "Child task {}(first is 0) of OsDependent {:?} failed {}",
                        index, self.os, error
                    ),
                    error
                );
            }
        }
        Ok(())
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
    fn execute(&self) -> Result<(), TaskError> {
        println!("{}", self.prompt);
        Ok(())
    }
}

impl Display for PrintTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
