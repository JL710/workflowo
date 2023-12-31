use crate::tasks::shell::{Bash, Cmd, ShellCommand};
use crate::tasks::ssh::{
    RemoteTransfer, ScpFileDownload, ScpFileUpload, SftpDownload, SftpUpload, SshCommand, SshTask,
};
use crate::tasks::{Job, OSDependent, PrintTask, Task, OS};
use serde_yaml::{self, Mapping, Value};
use std::fmt;
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

#[derive(Debug, Clone)]
struct ParsingError {
    message: String,
}

impl ParsingError {
    fn new(message: &str) -> Self {
        ParsingError {
            message: message.to_string(),
        }
    }

    fn from_string(message: String) -> Self {
        ParsingError { message }
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParsingError {{ {} }}", self.message)
    }
}

fn read_yaml_file(path: PathBuf) -> Value {
    let file = File::open(path).unwrap();
    let mut value: Value = serde_yaml::from_reader(file).unwrap();
    value.apply_merge().unwrap();
    value
}

fn parse_jobs(data: Mapping) -> Vec<Job> {
    let mut jobs = Vec::new();

    for (root_key, _root_value) in &data {
        if !root_key.is_string() {
            panic!("Job {:?} is has not a valid string as name", root_key);
        }

        if root_key.as_str().unwrap() == "IGNORE" {
            continue;
        }

        let job = parse_job(&data, root_key.as_str().unwrap().to_string());

        match job {
            Ok(value) => jobs.push(value),
            Err(error) => panic!("Error: {}", error),
        }
    }

    jobs
}

fn parse_job(root_map: &Mapping, name: String) -> Result<Job, ParsingError> {
    let job_entry = match get_entry(root_map, name.clone().into()) {
        Some(value) => value,
        _ => return Err(ParsingError::new("Job not found")),
    };

    let job_sequence = match job_entry.as_sequence() {
        Some(value) => value,
        None => {
            return Err(ParsingError::from_string(format!(
                "Child of {} is not a sequence",
                name
            )))
        }
    };

    let mut job = Job::new(name.clone());

    for child in job_sequence {
        match parse_task(root_map, child) {
            Ok(task) => job.add_child(task),
            Err(error) => {
                return Err(ParsingError::from_string(format!(
                    "Error while parsing job {}: {}",
                    name, error
                )))
            }
        }
    }
    Ok(job)
}

fn parse_task(root_map: &Mapping, value: &Value) -> Result<Box<dyn Task>, ParsingError> {
    if value.is_string() {
        match parse_job(root_map, value.as_str().unwrap().to_string()) {
            Ok(child_job) => {
                return Ok(Box::new(child_job));
            }
            Err(error) => {
                return Err(ParsingError::from_string(format!(
                    "parsing error for task {}: {}",
                    value.as_str().unwrap(),
                    error
                )))
            }
        }
    }

    if !value.is_mapping() {
        return Err(ParsingError::new(
            "Parsing error with task. Task is not of type Mapping!",
        ));
    }

    if let Some((task_key, task_value)) = value.as_mapping().unwrap().into_iter().next() {
        if !task_key.is_string() {
            return Err(ParsingError::new("task has an issue with the name"));
        }

        match task_key.as_str().unwrap() {
            "bash" => match parse_shell_command_task::<Bash>(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(err) => {
                    return Err(ParsingError::from_string(format!(
                        "Error with bash task: {}",
                        err
                    )))
                }
            },
            "cmd" => match parse_shell_command_task::<Cmd>(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(err) => {
                    return Err(ParsingError::from_string(format!(
                        "Error with cmd task: {}",
                        err
                    )))
                }
            },
            "on-windows" => match parse_os_dependent(root_map, OS::Windows, task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "parsing Error in on-windows: {}",
                        error
                    )))
                }
            },
            "on-linux" => match parse_os_dependent(root_map, OS::Linux, task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "parsing Error in on-windows: {}",
                        error
                    )))
                }
            },
            "ssh" => match parse_ssh(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in ssh: {}",
                        error
                    )))
                }
            },
            "scp-download" => match parse_remote_transfer::<ScpFileDownload>(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in scp-download: {}",
                        error
                    )))
                }
            },
            "scp-upload" => match parse_remote_transfer::<ScpFileUpload>(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in scp-upload: {}",
                        error
                    )))
                }
            },
            "sftp-download" => match parse_remote_transfer::<SftpDownload>(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in sftp-download: {}",
                        error
                    )))
                }
            },
            "sftp-upload" => match parse_remote_transfer::<SftpUpload>(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in sftp-upload: {}",
                        error
                    )))
                }
            },
            "print" => match parse_print(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in print: {}",
                        error
                    )));
                }
            },
            _ => return Err(ParsingError::new("unrecognized task in")),
        }
    }

    Err(ParsingError::new("Task could not be parsed"))
}

fn parse_print(value: &Value) -> Result<PrintTask, ParsingError> {
    match value {
        Value::String(prompt) => Ok(PrintTask::new(prompt.to_string())),
        other => Err(ParsingError::from_string(format!(
            "print value is not a string: {:?}",
            other
        ))),
    }
}

fn parse_remote_transfer<T: RemoteTransfer>(value: &Value) -> Result<T, ParsingError> {
    if !value.is_mapping() {
        return Err(ParsingError::new("Value is not of type Mapping"));
    }

    let username = match get_entry(value.as_mapping().unwrap(), "username".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => return Err(ParsingError::new("username is not a string")),
        },
        _ => return Err(ParsingError::new("username is not given")),
    };

    let password = match get_entry(value.as_mapping().unwrap(), "password".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => return Err(ParsingError::new("password is not a string")),
        },
        _ => return Err(ParsingError::new("password is not given")),
    };

    let address = match get_entry(value.as_mapping().unwrap(), "address".into()) {
        Some(value) => match value {
            Value::String(string) => match Ipv4Addr::from_str(&string) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            },
            _ => return Err(ParsingError::new("address is not a string")),
        },
        _ => return Err(ParsingError::new("address is not given")),
    };

    let remote_path = match get_entry(value.as_mapping().unwrap(), "remote_path".into()) {
        Some(value) => match value {
            Value::String(string) => match std::path::PathBuf::from_str(&string) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            },
            _ => return Err(ParsingError::new("remote_path is not a string")),
        },
        _ => return Err(ParsingError::new("remote_path is not given")),
    };

    let local_path = match get_entry(value.as_mapping().unwrap(), "local_path".into()) {
        Some(value) => match value {
            Value::String(string) => match std::path::PathBuf::from_str(&string) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            },
            _ => return Err(ParsingError::new("local_path is not a string")),
        },
        _ => return Err(ParsingError::new("local_path is not given")),
    };

    Ok(T::new(address, username, password, remote_path, local_path))
}

fn parse_ssh(value: &Value) -> Result<SshTask, ParsingError> {
    if !value.is_mapping() {
        return Err(ParsingError::new("Value is not of type Mapping"));
    }

    let username = match get_entry(value.as_mapping().unwrap(), "username".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => return Err(ParsingError::new("username is not a string")),
        },
        _ => return Err(ParsingError::new("username is not given")),
    };

    let password = match get_entry(value.as_mapping().unwrap(), "password".into()) {
        Some(value) => match value {
            Value::String(string) => string,
            _ => return Err(ParsingError::new("password is not a string")),
        },
        _ => return Err(ParsingError::new("password is not given")),
    };

    let address = match get_entry(value.as_mapping().unwrap(), "address".into()) {
        Some(value) => match value {
            Value::String(string) => match Ipv4Addr::from_str(&string) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            },
            _ => return Err(ParsingError::new("address is not a string")),
        },
        _ => return Err(ParsingError::new("address is not given")),
    };

    let command_sequence = match get_entry(value.as_mapping().unwrap(), "commands".into()) {
        Some(value) => {
            if !value.is_sequence() {
                return Err(ParsingError::new("commands are not a sequence"));
            }
            value.as_sequence().unwrap().clone()
        }
        _ => return Err(ParsingError::new("commands are not given")),
    };

    let mut commands = Vec::new();
    for item in command_sequence {
        commands.push(parse_ssh_command(&item)?);
    }

    Ok(SshTask::new(address, username, password, commands))
}

fn parse_ssh_command(value: &Value) -> Result<SshCommand, ParsingError> {
    match value {
        Value::String(string) => Ok(SshCommand::new(string.to_owned(), vec![0])),
        Value::Mapping(map) => {
            let command_map = match get_entry(map, "command".into()) {
                Some(entry_value) => {
                    if !entry_value.is_mapping() {
                        return Err(ParsingError::new("Ssh command is not a map"));
                    }
                    entry_value.as_mapping().unwrap().to_owned()
                }
                _ => {
                    return Err(ParsingError::new(
                        "Ssh command has misleading key. Expected 'command'",
                    ))
                }
            };
            let command = match get_entry(&command_map, "command".into()) {
                Some(command_entry) => {
                    if !command_entry.is_string() {
                        return Err(ParsingError::new("Ssh command is not a string"));
                    }
                    command_entry.as_str().unwrap().to_owned()
                }
                _ => {
                    return Err(ParsingError::new(
                        "Ssh command missing key. Expected 'command'",
                    ))
                }
            };
            let exit_codes_sequence = match get_entry(&command_map, "exit_codes".into()) {
                Some(exit_codes_entry) => {
                    if !exit_codes_entry.is_sequence() {
                        return Err(ParsingError::new(
                            "Ssh command exit_codes is not a sequence",
                        ));
                    }
                    exit_codes_entry.as_sequence().unwrap().to_owned()
                }
                _ => {
                    return Err(ParsingError::new(
                        "Ssh command missing key. Expected 'command'",
                    ))
                }
            };
            let mut exit_codes: Vec<i32> = Vec::new();
            for exit_code_value in exit_codes_sequence {
                if !exit_code_value.is_number() {
                    return Err(ParsingError::from_string(format!(
                        "Ssh command exit_code {:?} is not a number",
                        exit_code_value
                    )));
                }
                exit_codes.push(exit_code_value.as_i64().unwrap() as i32);
            }
            Ok(SshCommand::new(command, exit_codes))
        }
        _ => Err(ParsingError::from_string(format!(
            "command is not a string: {:?}",
            value
        ))),
    }
}

fn parse_os_dependent(
    root_map: &Mapping,
    os: OS,
    value: &Value,
) -> Result<OSDependent, ParsingError> {
    if !value.is_sequence() {
        return Err(ParsingError::new("value is not a sequence"));
    }

    let mut task = OSDependent::new(os);
    for child_item in value.as_sequence().unwrap() {
        match parse_task(root_map, child_item) {
            Ok(child_task) => task.add_child(child_task),
            Err(error) => {
                return Err(ParsingError::from_string(format!(
                    "could not parse child task for {}: {}",
                    task, error
                )))
            }
        }
    }

    Ok(task)
}

fn parse_shell_command_task<T: ShellCommand>(value: &Value) -> Result<T, ParsingError> {
    match value {
        Value::Mapping(cmd_map) => {
            let command_value = match get_entry(cmd_map, "command".into()) {
                Some(entry) => match entry {
                    Value::String(string) => string,
                    _ => return Err(ParsingError::new("command is not a string")),
                },
                _ => return Err(ParsingError::new("command is not given")),
            };

            let work_dir_value = match get_entry(cmd_map, "work_dir".into()) {
                Some(entry) => match entry {
                    Value::String(string) => Some(string),
                    _ => return Err(ParsingError::new("command is not a string")),
                },
                _ => None,
            };

            return Ok(T::new(
                command_value.split(' ').map(|x| x.to_string()).collect(),
                work_dir_value,
            ));
        }
        val => match val {
            // case it is just the string shortcut `bash: "somestring"`
            Value::String(string) => {
                return Ok(T::new(
                    string.split(' ').map(|x| x.to_string()).collect(),
                    None,
                ));
            }
            _ => Err(ParsingError::new("task has a problem with its definition")),
        },
    }
}

/// Parses the file and returns a vector of the found jobs.
pub fn jobs_from_file(path: PathBuf) -> Vec<Job> {
    let mut value = read_yaml_file(path);
    render::render(&mut std::collections::HashMap::new(), &mut value); // pre render everything
    parse_jobs(value.as_mapping().unwrap().to_owned())
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
