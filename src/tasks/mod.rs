use std::{env, fmt, fmt::Display};
pub mod shell;
pub mod ssh;

/// task_panic!("Message") -> Result<(), TaskError>
macro_rules! task_panic {
    ($message:expr) => {
        return Err(TaskError::from_message($message.to_string()));
    };
}
pub(crate) use task_panic;

/// task_error_panic!("message", error) -> Result<(), TaskError>
macro_rules! task_error_panic {
    ($message:expr, $error:expr) => {
        return Err(TaskError::from_error(
            $message.to_string(),
            Box::new($error),
        ))
    };
}
pub(crate) use task_error_panic;

/// Will take a piece of code and a message.
/// If the executed code returns Ok(value) the macro returns the value.
/// If Err gets `returned task_error_panic` gets called with the message and the error.
macro_rules! task_might_panic {
    ($code:expr, $message:expr) => {
        match $code {
            Ok(value) => value,
            Err(error) => task_error_panic!($message, error),
        }
    };
}
pub(crate) use task_might_panic;

#[derive(Debug)]
pub struct TaskError {
    message: String,
    source_error: Option<Box<dyn std::error::Error>>,
}

impl TaskError {
    fn new(message: String, source_error: Option<Box<dyn std::error::Error>>) -> Self {
        Self {
            message,
            source_error,
        }
    }

    fn from_error(message: String, source_error: Box<dyn std::error::Error>) -> Self {
        Self {
            message,
            source_error: Some(source_error),
        }
    }

    fn from_message(message: String) -> Self {
        Self {
            message,
            source_error: None,
        }
    }
}

impl Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TaskError {}. Source: {:?}",
            self.message, self.source_error
        )
    }
}

impl From<Box<dyn std::error::Error>> for TaskError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self {
            message: value.to_string(),
            source_error: Some(value),
        }
    }
}

impl std::error::Error for TaskError {}

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
        for child in self.children.iter() {
            if let Err(error) = child.execute() {
                task_error_panic!(
                    format!("Child task of {} failed with {:?}", &self.name, error),
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

        for child in &self.children {
            if let Err(error) = child.execute() {
                task_error_panic!(
                    format!(
                        "Child task of OsDependent {:?} failed with {:?}",
                        self.os, error
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
