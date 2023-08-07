use super::{task_dynerror_panic, task_might_panic, task_panic, Task, TaskError, SourceError};
use std::fmt::{self, Display};
use std::process::Command;

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
    fn execute(&self) -> Result<(), TaskError> {
        let mut command = Command::new("bash");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = task_might_panic!(
            command.arg("-c").arg(&self.args.join(" ")).output(),
            format!("Failed while executing bash command")
        );
        if !output.status.success() {
            task_panic!(format!(
                "Error: {:?} did not success and raised an error!\n{}",
                &self.args,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
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
    fn execute(&self) -> Result<(), TaskError> {
        let mut command = Command::new("cmd");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = task_might_panic!(
            command.arg("/c").args(&self.args).output(),
            "Failed while cmd execution"
        );
        if !output.status.success() {
            task_panic!(format!(
                "Error: {:?} did not success and raised an error!\n{}",
                &self.args,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}

impl Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
