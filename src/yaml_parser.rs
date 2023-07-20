use crate::{Bash, Cmd, Job, OSDependent, ScpFileDownload, ShellCommand, SshCommand, Task, OS};
use serde_yaml::{self, Mapping, Value};
use std::fmt;
use std::fs::File;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;

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

fn read_yaml_file(path: PathBuf) -> Mapping {
    let file = File::open(path).unwrap();
    let value: Mapping = serde_yaml::from_reader(file).unwrap();
    value
}

fn parse_jobs(data: Mapping) -> Vec<Job> {
    let mut jobs = Vec::new();

    for (root_key, _root_value) in &data {
        if !root_key.is_string() {
            panic!("Job {:?} is has not a valid string as name", root_key);
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
    let job_entry = match root_map.clone().entry(name.clone().into()) {
        serde_yaml::mapping::Entry::Occupied(value) => value.get().clone(),
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

    let mut job = Job {
        name: name.clone(),
        children: Vec::new(),
    };

    for child in job_sequence {
        match parse_task(root_map, child) {
            Ok(task) => job.children.push(task),
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
            "scp-download" => match parse_scp_file_download(task_value) {
                Ok(task) => return Ok(Box::new(task)),
                Err(error) => {
                    return Err(ParsingError::from_string(format!(
                        "Parsing Error in ssh: {}",
                        error
                    )))
                }
            },
            _ => return Err(ParsingError::new("unrecognized task in")),
        }
    }

    Err(ParsingError::new("Task could not be parsed"))
}

fn parse_scp_file_download(value: &Value) -> Result<ScpFileDownload, ParsingError> {
    if !value.is_mapping() {
        return Err(ParsingError::new("Value is not of type Mapping"));
    }

    let username = match value.as_mapping().unwrap().clone().entry("username".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("username is not a string"));
            }
            value.get().as_str().unwrap().to_string()
        }
        _ => return Err(ParsingError::new("username is not given")),
    };

    let password = match value.as_mapping().unwrap().clone().entry("password".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("password is not a string"));
            }
            value.get().as_str().unwrap().to_string()
        }
        _ => return Err(ParsingError::new("password is not given")),
    };

    let address = match value.as_mapping().unwrap().clone().entry("address".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("address is not a string"));
            }
            match Ipv4Addr::from_str(value.get().as_str().unwrap()) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            }
        }
        _ => return Err(ParsingError::new("address is not given")),
    };

    let remote_path = match value
        .as_mapping()
        .unwrap()
        .clone()
        .entry("remote_path".into())
    {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("remote_path is not a string"));
            }
            match std::path::PathBuf::from_str(value.get().as_str().unwrap()) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            }
        }
        _ => return Err(ParsingError::new("remote_path is not given")),
    };

    let local_path = match value
        .as_mapping()
        .unwrap()
        .clone()
        .entry("local_path".into())
    {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("local_path is not a string"));
            }
            match std::path::PathBuf::from_str(value.get().as_str().unwrap()) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            }
        }
        _ => return Err(ParsingError::new("local_path is not given")),
    };

    Ok(ScpFileDownload {
        address,
        user: username,
        password,
        remote_path,
        local_path,
    })
}

fn parse_ssh(value: &Value) -> Result<SshCommand, ParsingError> {
    if !value.is_mapping() {
        return Err(ParsingError::new("Value is not of type Mapping"));
    }

    let username = match value.as_mapping().unwrap().clone().entry("username".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("username is not a string"));
            }
            value.get().as_str().unwrap().to_string()
        }
        _ => return Err(ParsingError::new("username is not given")),
    };

    let password = match value.as_mapping().unwrap().clone().entry("password".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("password is not a string"));
            }
            value.get().as_str().unwrap().to_string()
        }
        _ => return Err(ParsingError::new("password is not given")),
    };

    let address = match value.as_mapping().unwrap().clone().entry("address".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_string() {
                return Err(ParsingError::new("address is not a string"));
            }
            match Ipv4Addr::from_str(value.get().as_str().unwrap()) {
                Ok(value) => value,
                Err(error) => return Err(ParsingError::from_string(error.to_string())),
            }
        }
        _ => return Err(ParsingError::new("address is not given")),
    };

    let command_sequence = match value.as_mapping().unwrap().clone().entry("commands".into()) {
        serde_yaml::mapping::Entry::Occupied(value) => {
            if !value.get().is_sequence() {
                return Err(ParsingError::new("commands are not a sequence"));
            }
            value.get().as_sequence().unwrap().clone()
        }
        _ => return Err(ParsingError::new("commands are not given")),
    };

    let mut commands = Vec::new();
    for item in command_sequence {
        if !item.is_string() {
            return Err(ParsingError::from_string(format!(
                "command is not a string: {:?}",
                item
            )));
        }
        commands.push(item.as_str().unwrap().to_owned());
    }

    Ok(SshCommand {
        address,
        password,
        user: username,
        commands,
    })
}

fn parse_os_dependent(
    root_map: &Mapping,
    os: OS,
    value: &Value,
) -> Result<OSDependent, ParsingError> {
    if !value.is_sequence() {
        return Err(ParsingError::new("value is not a sequence"));
    }

    let mut task = OSDependent {
        os,
        children: Vec::new(),
    };
    for child_item in value.as_sequence().unwrap() {
        match parse_task(root_map, child_item) {
            Ok(child_task) => task.children.push(child_task),
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
        Value::String(content) => {
            return Ok(T::new(
                content
                    .split(' ')
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect(),
                None,
            ));
        }
        Value::Mapping(cmd_map) => {
            let command_value = match cmd_map.clone().entry("command".into()) {
                serde_yaml::mapping::Entry::Occupied(value) => {
                    Some(value.get().clone().as_str().unwrap().to_string())
                }
                _ => None,
            };
            let work_dir_value = match cmd_map.clone().entry("work_dir".into()) {
                serde_yaml::mapping::Entry::Occupied(value) => {
                    Some(value.get().clone().as_str().unwrap().to_string())
                }
                _ => None,
            };

            if let Some(value) = command_value {
                return Ok(T::new(
                    value
                        .split(' ')
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                    work_dir_value,
                ));
            } else {
                Err(ParsingError::new("Command not given"))
            }
        }
        _ => Err(ParsingError::new("task has a problem with its definition")),
    }
}

/// Parses the file and returns a vector of the found jobs.
pub fn jobs_from_file(path: PathBuf) -> Vec<Job> {
    parse_jobs(read_yaml_file(path))
}
