use super::Task;
use anyhow::{bail, Context, Result};
use std::fmt::{self, Display};
use std::process::Command;

pub trait ShellCommand {
    fn new(
        args: Vec<String>,
        work_dir: Option<String>,
        allowed_exit_codes: Option<Vec<i32>>,
    ) -> Self;
}

#[derive(Debug)]
pub struct Bash {
    args: Vec<String>,
    work_dir: Option<String>,
    allowed_exit_codes: Option<Vec<i32>>,
}

impl ShellCommand for Bash {
    fn new(
        args: Vec<String>,
        work_dir: Option<String>,
        allowed_exit_codes: Option<Vec<i32>>,
    ) -> Self {
        Bash {
            args,
            work_dir,
            allowed_exit_codes,
        }
    }
}

impl Task for Bash {
    fn execute(&self) -> Result<()> {
        let mut command = Command::new("bash");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = command
            .arg("-c")
            .arg(&self.args.join(" "))
            .output()
            .context("Failed while executing bash command")?;
        let exit_code = output
            .status
            .code()
            .context("process did not return an exit code")?;
        if match &self.allowed_exit_codes {
            Some(codes) => !codes.contains(&exit_code),
            None => exit_code != 0,
        } {
            bail!(format!(
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
    allowed_exit_codes: Option<Vec<i32>>,
}

impl ShellCommand for Cmd {
    fn new(
        args: Vec<String>,
        work_dir: Option<String>,
        allowed_exit_codes: Option<Vec<i32>>,
    ) -> Self {
        Cmd {
            args,
            work_dir,
            allowed_exit_codes,
        }
    }
}

impl Task for Cmd {
    fn execute(&self) -> Result<()> {
        let mut command = Command::new("cmd");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = command
            .arg("/c")
            .args(&self.args)
            .output()
            .context("Failed while cmd execution")?;
        let exit_code = output
            .status
            .code()
            .context("process did not return an exit code")?;
        if match &self.allowed_exit_codes {
            Some(codes) => !codes.contains(&exit_code),
            None => exit_code != 0,
        } {
            bail!(format!(
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
