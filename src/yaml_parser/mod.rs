use crate::tasks::shell::{Bash, Cmd, ShellCommand};
use crate::tasks::ssh::{
    RemoteTransfer, ScpFileDownload, ScpFileUpload, SftpDownload, SftpUpload, SshCommand, SshTask,
};
use crate::tasks::{Job, OSDependent, ParallelTask, PrintTask, Task, OS};
use anyhow::{bail, Context, Result};
use serde_yaml::{self, Mapping, Value};
use std::fs::File;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
mod render;

/// Gets an entry out of a map.
fn get_entry(map: &Mapping, key: Value) -> Option<Value> {
    match map.clone().entry(key) {
        serde_yaml::mapping::Entry::Occupied(value) => Some(value.get().to_owned()),
        _ => None,
    }
}

fn read_yaml_file(path: PathBuf) -> Result<Value> {
    let file = File::open(path).context("Error while opening file")?;
    let mut value: Value = serde_yaml::from_reader(file).context("Incorrect Yaml")?;
    value.apply_merge().context("Merging yaml values error")?;
    Ok(value)
}

fn parse_jobs(data: Mapping) -> Result<Vec<Job>> {
    let mut jobs = Vec::new();

    for (root_key, _root_value) in &data {
        if !root_key.is_string() {
            bail!("Job {:?} is has not a valid string as name", root_key);
        }

        if root_key.as_str().unwrap() == "IGNORE" {
            continue;
        }
        jobs.push(
            parse_job(&data, root_key.as_str().unwrap().to_string())
                .context("parsing job failed")?,
        );
    }

    Ok(jobs)
}

fn parse_job(root_map: &Mapping, name: String) -> Result<Job> {
    let job_entry = match get_entry(root_map, name.clone().into()) {
        Some(value) => value,
        _ => bail!("Job not found"),
    };

    let job_sequence = match job_entry.as_sequence() {
        Some(value) => value,
        None => {
            bail!(format!("Child of {} is not a sequence", name));
        }
    };

    let mut job = Job::new(name.clone());

    for child in job_sequence {
        job.add_child(
            parse_task(root_map, child).context(format!("Error while parsing job {}", name))?,
        );
    }
    Ok(job)
}

fn parse_task(root_map: &Mapping, value: &Value) -> Result<Box<dyn Task>> {
    if value.is_string() {
        match parse_job(root_map, value.as_str().unwrap().to_string()) {
            Ok(child_job) => {
                return Ok(Box::new(child_job));
            }
            Err(error) => {
                bail!(format!(
                    "parsing error for task {}: {}",
                    value.as_str().unwrap(),
                    error
                ))
            }
        }
    }

    if !value.is_mapping() {
        bail!("Parsing error with task. Task is not of type Mapping!");
    }

    if let Some((task_key, task_value)) = value.as_mapping().unwrap().into_iter().next() {
        if !task_key.is_string() {
            bail!("task has an issue with the name");
        }

        match task_key.as_str().unwrap() {
            "bash" => {
                return Ok(Box::new(
                    parse_shell_command_task::<Bash>(task_value)
                        .context("parsing error with bash task")?,
                ))
            }
            "cmd" => {
                return Ok(Box::new(
                    parse_shell_command_task::<Cmd>(task_value)
                        .context("parsing error with cmd task")?,
                ))
            }
            "on-windows" => {
                return Ok(Box::new(
                    parse_os_dependent(root_map, OS::Windows, task_value)
                        .context("parsing error in on-window")?,
                ))
            }
            "on-linux" => {
                return Ok(Box::new(
                    parse_os_dependent(root_map, OS::Linux, task_value)
                        .context("parsing error in on-linux")?,
                ))
            }
            "ssh" => {
                return Ok(Box::new(
                    parse_ssh(task_value).context("parsing error in ssh")?,
                ))
            }
            "scp-download" => {
                return Ok(Box::new(
                    parse_remote_transfer::<ScpFileDownload>(task_value)
                        .context("parsing error in scp-download")?,
                ))
            }
            "scp-upload" => {
                return Ok(Box::new(
                    parse_remote_transfer::<ScpFileUpload>(task_value)
                        .context("parsing error in scp-upload")?,
                ))
            }
            "sftp-download" => {
                return Ok(Box::new(
                    parse_remote_transfer::<SftpDownload>(task_value)
                        .context("parsing error in sftp-download")?,
                ))
            }
            "sftp-upload" => {
                return Ok(Box::new(
                    parse_remote_transfer::<SftpUpload>(task_value)
                        .context("parsing error in sftp-upload")?,
                ))
            }
            "print" => {
                return Ok(Box::new(
                    parse_print(task_value).context("parsing error in print")?,
                ))
            }
            "parallel" => {
                return Ok(Box::new(
                    parse_parallel_task(root_map, task_value)
                        .context("parsing error in parallel task")?,
                ))
            }
            task_name => bail!("unrecognized task {}", task_name),
        }
    }

    bail!("task could not be parsed");
}

fn parse_parallel_task(root_map: &Mapping, value: &Value) -> Result<ParallelTask> {
    let mut threads = (std::thread::available_parallelism()
        .context("failed to estimate best thread amount")?
        .get()
        - 1) as u8; // -1 because of main thread
    let mut tasks = Vec::new();

    let task_seq = match value {
        Value::Sequence(seq) => seq.to_owned(),
        Value::Mapping(map) => {
            // get threads number
            if let Some(thread_value) = get_entry(map, "threads".into()) {
                if thread_value.is_u64() {
                    threads = thread_value.as_u64().unwrap() as u8;
                } else {
                    bail!("threads value of parallel task is not a valid number");
                }
            }
            // get/return task seq
            match get_entry(map, "tasks".into())
                .context("tasks was not provided to parallel task")?
            {
                Value::Sequence(seq) => seq,
                _ => bail!(""),
            }
        }
        _ => bail!("parallel task needs to be a sequence or mapping but is not"),
    };
    if task_seq.is_empty() {
        bail!("task sequence has no entries");
    }
    for item in task_seq {
        tasks.push(
            parse_task(root_map, &item).context("failed to subtask parse task of parallel task")?,
        );
    }

    Ok(ParallelTask::new(tasks, threads))
}

fn parse_print(value: &Value) -> Result<PrintTask> {
    match value {
        Value::String(prompt) => Ok(PrintTask::new(prompt.to_string())),
        other => bail!(format!("print value is not a string: {:?}", other)),
    }
}

fn parse_remote_transfer<T: RemoteTransfer>(value: &Value) -> Result<T> {
    if !value.is_mapping() {
        bail!("Value is not of type Mapping");
    }

    let username = match get_entry(value.as_mapping().unwrap(), "username".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => bail!("username is not a string"),
        },
        _ => bail!("username is not given"),
    };

    let password = match get_entry(value.as_mapping().unwrap(), "password".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => bail!("password is not a string"),
        },
        _ => bail!("password is not given"),
    };

    let address = match get_entry(value.as_mapping().unwrap(), "address".into()) {
        Some(value) => match value {
            Value::String(string) => match Ipv4Addr::from_str(&string) {
                Ok(value) => value,
                Err(error) => bail!(error.to_string()),
            },
            _ => bail!("address is not a string"),
        },
        _ => bail!("address is not given"),
    };

    let remote_path = match get_entry(value.as_mapping().unwrap(), "remote_path".into()) {
        Some(value) => match value {
            Value::String(string) => match std::path::PathBuf::from_str(&string) {
                Ok(value) => value,
                Err(error) => bail!(error.to_string()),
            },
            _ => bail!("remote_path is not a string"),
        },
        _ => bail!("remote_path is not given"),
    };

    let local_path = match get_entry(value.as_mapping().unwrap(), "local_path".into()) {
        Some(value) => match value {
            Value::String(string) => match std::path::PathBuf::from_str(&string) {
                Ok(value) => value,
                Err(error) => bail!(error.to_string()),
            },
            _ => bail!("local_path is not a string"),
        },
        _ => bail!("local_path is not given"),
    };

    T::new(address, username, password, remote_path, local_path)
        .context("Could not create Task for remote transfer operation")
}

fn parse_ssh(value: &Value) -> Result<SshTask> {
    if !value.is_mapping() {
        bail!("Value is not of type Mapping");
    }

    let username = match get_entry(value.as_mapping().unwrap(), "username".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => bail!("username is not a string"),
        },
        _ => bail!("username is not given"),
    };

    let password = match get_entry(value.as_mapping().unwrap(), "password".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => bail!("password is not a string"),
        },
        _ => bail!("password is not given"),
    };

    let address = match get_entry(value.as_mapping().unwrap(), "address".into()) {
        Some(value) => match value {
            Value::String(string) => Ipv4Addr::from_str(&string)?,
            _ => bail!("address is not a string"),
        },
        _ => bail!("address is not given"),
    };

    let command_sequence = match get_entry(value.as_mapping().unwrap(), "commands".into()) {
        Some(value) => {
            if !value.is_sequence() {
                bail!("commands are not a sequence");
            }
            value.as_sequence().unwrap().clone()
        }
        _ => bail!("commands are not given"),
    };

    let mut commands = Vec::new();
    for item in command_sequence {
        commands.push(parse_ssh_command(&item).context("parsing of ssh command failed")?);
    }

    Ok(SshTask::new(address, username, password, commands))
}

fn parse_ssh_command(value: &Value) -> Result<SshCommand> {
    match value {
        Value::String(string) => Ok(SshCommand::new(string.to_owned(), vec![0])),
        Value::Mapping(map) => {
            let command_map = match get_entry(map, "command".into()) {
                Some(entry_value) => {
                    if !entry_value.is_mapping() {
                        bail!("Ssh command is not a map");
                    }
                    entry_value.as_mapping().unwrap().to_owned()
                }
                _ => {
                    bail!("Ssh command has misleading key. Expected 'command'",)
                }
            };
            let command = match get_entry(&command_map, "command".into()) {
                Some(command_entry) => {
                    if !command_entry.is_string() {
                        bail!("Ssh command is not a string");
                    }
                    command_entry.as_str().unwrap().to_owned()
                }
                _ => {
                    bail!("Ssh command missing key. Expected 'command'",)
                }
            };
            let exit_codes_sequence = match get_entry(&command_map, "exit_codes".into()) {
                Some(exit_codes_entry) => {
                    if !exit_codes_entry.is_sequence() {
                        bail!("Ssh command exit_codes is not a sequence");
                    }
                    exit_codes_entry.as_sequence().unwrap().to_owned()
                }
                _ => {
                    bail!("Ssh command missing key. Expected 'command'")
                }
            };
            let mut exit_codes: Vec<i32> = Vec::new();
            for exit_code_value in exit_codes_sequence {
                if !exit_code_value.is_number() {
                    bail!(format!(
                        "Ssh command exit_code {:?} is not a number",
                        exit_code_value
                    ));
                }
                exit_codes.push(exit_code_value.as_i64().unwrap() as i32);
            }
            Ok(SshCommand::new(command, exit_codes))
        }
        _ => bail!(format!("command is not a string: {:?}", value)),
    }
}

fn parse_os_dependent(root_map: &Mapping, os: OS, value: &Value) -> Result<OSDependent> {
    if !value.is_sequence() {
        bail!("value is not a sequence");
    }

    let mut task = OSDependent::new(os);
    for child_item in value.as_sequence().unwrap() {
        task.add_child(
            parse_task(root_map, child_item)
                .context(format!("could not parse child task for {}", task))?,
        );
    }

    Ok(task)
}

fn parse_shell_command_task<T: ShellCommand>(value: &Value) -> Result<T> {
    match value {
        Value::Mapping(cmd_map) => {
            let command_value = match get_entry(cmd_map, "command".into()) {
                Some(entry) => match entry {
                    Value::String(string) => string,
                    _ => bail!("command is not a string"),
                },
                _ => bail!("command is not given"),
            };

            let work_dir_value = match get_entry(cmd_map, "work_dir".into()) {
                Some(entry) => match entry {
                    Value::String(string) => Some(string),
                    _ => bail!("command is not a string"),
                },
                _ => None,
            };

            let allowed_exit_codes = match get_entry(cmd_map, "exit_codes".into()) {
                Some(entry) => match entry {
                    Value::Sequence(seq) => {
                        if seq.is_empty() {
                            bail!("no exit codes are provided");
                        }
                        let mut codes = Vec::new();
                        for val in seq {
                            if let Value::Number(num) = val {
                                codes.push(
                                    num.as_i64().context("could not convert exit code to i64")?
                                        as i32,
                                );
                            } else {
                                bail!("exit code is not a number");
                            }
                        }
                        Some(codes)
                    }
                    _ => bail!("allowed exit codes is not a sequence"),
                },
                _ => None,
            };

            return Ok(T::new(
                command_value.split(' ').map(|x| x.to_string()).collect(),
                work_dir_value,
                allowed_exit_codes,
            ));
        }
        val => match val {
            // case it is just the string shortcut `bash: "somestring"`
            Value::String(string) => {
                return Ok(T::new(
                    string.split(' ').map(|x| x.to_string()).collect(),
                    None,
                    None,
                ));
            }
            _ => bail!("task has a problem with its definition"),
        },
    }
}

/// Parses the file and returns a vector of the found jobs.
pub fn jobs_from_file(path: PathBuf) -> Result<Vec<Job>> {
    let mut value = read_yaml_file(path).context("reading yaml error")?;
    render::render(&mut std::collections::HashMap::new(), &mut value)
        .context("resolving yaml error")?; // pre render everything
    parse_jobs(value.as_mapping().unwrap().to_owned()).context("failed to parse jobs in file")
}

#[cfg(test)]
mod tests {
    use serde_yaml::Value;

    use crate::{tasks::ssh::SshCommand, yaml_parser::parse_ssh_command};

    #[test]
    fn parse_ssh_command_test_simple() {
        let value: Value = serde_yaml::from_str(
            "
        'ls 1'
        ",
        )
        .unwrap();
        assert_eq!(
            parse_ssh_command(&value).unwrap(),
            SshCommand::new("ls 1".to_string(), vec![0])
        );
    }

    #[test]
    fn parse_ssh_command_test_exit_code() {
        let value: Value = serde_yaml::from_str(
            "
        command:
            command: 'ls 2'
            exit_codes: [1, 2, 3, 4, 5]
        ",
        )
        .unwrap();
        assert_eq!(
            parse_ssh_command(&value).unwrap(),
            SshCommand::new("ls 2".to_string(), vec![1, 2, 3, 4, 5])
        );
    }
}
