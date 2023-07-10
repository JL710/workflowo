use crate::{Bash, Cmd, Job, ShellCommand};
use serde_yaml::{self, Mapping, Value};
use std::fmt;
use std::fs::File;
use std::path::PathBuf;

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
    let jobs = Vec::new();

    for (root_key, root_value) in data {
        if !root_value.is_sequence() {
            panic!(
                "Parsing error with Job {}. Child is not of type Sequence!",
                root_key.as_str().unwrap()
            );
        }

        let mut job = Job {
            name: root_key.as_str().unwrap().to_string(),
            children: Vec::new(),
        };

        for child_item in root_value.as_sequence().unwrap() {
            if !child_item.is_mapping() {
                panic!(
                    "Parsing error with child of {}. Child is not of type Mapping!",
                    root_key.as_str().unwrap()
                );
            }

            for (task_key, task_value) in child_item.as_mapping().unwrap() {
                if !task_key.is_string() {
                    panic!(
                        "task in job {} has an issue with the name",
                        root_key.as_str().unwrap()
                    );
                }

                match task_key.as_str().unwrap() {
                    "bash" => match parse_shell_command_task::<Bash>(task_value) {
                        Ok(task) => job.children.push(Box::new(task)),
                        Err(err) => panic!("Error in job {}: {}", root_key.as_str().unwrap(), err),
                    },
                    "cmd" => match parse_shell_command_task::<Cmd>(task_value) {
                        Ok(task) => job.children.push(Box::new(task)),
                        Err(err) => panic!("Error in job {}: {}", root_key.as_str().unwrap(), err),
                    },
                    _ => panic!("unrecognized task in {}", root_key.as_str().unwrap()),
                }
            }
        }

        println!("{}", job);
    }

    jobs
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
        Value::Mapping(cmd_map) => match cmd_map.clone().entry("command".into()) {
            serde_yaml::mapping::Entry::Vacant(_) => {
                Err(ParsingError::new("Command not given"))
            }
            serde_yaml::mapping::Entry::Occupied(command_value) => {
                todo!("workdir");
                return Ok(T::new(
                    command_value
                        .get()
                        .as_str()
                        .unwrap()
                        .split(' ')
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                    None,
                ));
            }
        },
        _ => Err(ParsingError::new(
            "cmd task in has a problem with its definition",
        )),
    }
}

pub fn jobs_from_file(path: PathBuf) -> Vec<Job> {
    parse_jobs(read_yaml_file(path))
}
