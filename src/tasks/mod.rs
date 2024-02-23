use std::{env, fmt, fmt::Display};
pub mod shell;
pub mod ssh;
use anyhow::{Context, Result};
use std::sync::Arc;

pub trait Task: Display + Sync + Send {
    /// Will be called when the task should be executed.
    fn execute(&self) -> Result<()>;
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
    fn execute(&self) -> Result<()> {
        for (index, child) in self.children.iter().enumerate() {
            child.execute().context(format!(
                "Child {}(first is 0) of task {} failed",
                index, &self.name
            ))?;
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
    fn execute(&self) -> Result<()> {
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
            child.execute().context(format!(
                "Child task {}(first is 0) of OsDependent {:?} failed",
                index, self.os
            ))?;
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
    fn execute(&self) -> Result<()> {
        println!("{}", self.prompt);
        Ok(())
    }
}

impl Display for PrintTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct ParallelTask {
    tasks: Vec<Arc<Box<dyn Task>>>,
    threads: u8,
}

impl ParallelTask {
    pub fn new(tasks: Vec<Box<dyn Task>>, threads: u8) -> Self {
        let mut new_tasks = Vec::new();
        for task in tasks {
            new_tasks.push(Arc::new(task));
        }
        Self {
            tasks: new_tasks,
            threads,
        }
    }
}

impl Task for ParallelTask {
    fn execute(&self) -> Result<()> {
        let pool = threadpool::ThreadPool::new(self.threads as usize);

        let (tx, rx) = std::sync::mpsc::channel();

        for task in &self.tasks {
            let t = task.clone();
            let sender = tx.clone();
            pool.execute(move || {
                let result = t.execute();
                sender.send(result).unwrap();
            });
        }

        for _ in 0..self.tasks.len() {
            rx.recv()
                .context("receiving of thread result failed")?
                .context("Task of parallel task failed")?;
        }

        pool.join();

        Ok(())
    }
}

impl Display for ParallelTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = String::new();
        for task in &self.tasks {
            text.push_str(&format!("{},", task));
        }

        write!(
            f,
            "ParallelTask: {{ threads: {} tasks: {{ {} }} }}",
            self.threads, text
        )
    }
}
