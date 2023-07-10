use crate::{Bash, Cmd, Job};
use serde_yaml::{self, Mapping, Value};
use std::fs::File;
use std::path::PathBuf;

fn read_yaml_file(path: PathBuf) -> Mapping {
    let file = File::open(path).unwrap();
    let value: Mapping = serde_yaml::from_reader(file).unwrap();
    value
}

fn parse_jobs(data: Mapping) -> Vec<Job> {
    let jobs = Vec::new();

    for (key, value) in data {
        if !value.is_sequence() {
            panic!(
                "Parsing error with Job {}. Child is not of type Sequence!",
                key.as_str().unwrap()
            );
        }

        let mut job = Job {
            name: key.as_str().unwrap().to_string(),
            children: Vec::new(),
        };

        for child_item in value.as_sequence().unwrap() {
            if !child_item.is_mapping() {
                panic!(
                    "Parsing error with child of {}. Child is not of type Mapping!",
                    key.as_str().unwrap()
                );
            }

            match child_item
                .as_mapping()
                .unwrap()
                .keys()
                .next()
                .unwrap()
                .as_str()
                .unwrap()
            {
                "bash" => {
                    if let serde_yaml::mapping::Entry::Occupied(existing_entry) = child_item
                        .as_mapping()
                        .unwrap()
                        .clone()
                        .entry("bash".into())
                    {
                        match existing_entry.get() {
                            Value::String(content) => {
                                job.children.push(Box::new(Bash {
                                    args: content
                                        .split(' ')
                                        .into_iter()
                                        .map(|x| x.to_string())
                                        .collect(),
                                    work_dir: None,
                                }));
                            }
                            Value::Mapping(bash_map) => {
                                match bash_map.clone().entry("command".into()) {
                                    serde_yaml::mapping::Entry::Vacant(_) => {
                                        panic!("Command not given!")
                                    }
                                    serde_yaml::mapping::Entry::Occupied(command_value) => {
                                        job.children.push(Box::new(Bash {
                                            args: command_value
                                                .get()
                                                .as_str()
                                                .unwrap()
                                                .split(' ')
                                                .into_iter()
                                                .map(|x| x.to_string())
                                                .collect(),
                                            work_dir: None,
                                        }));
                                    }
                                }
                                todo!("workdir")
                            }
                            _ => {
                                todo!("Erro message");
                            }
                        }
                    }
                }
                "cmd" => {
                    if let serde_yaml::mapping::Entry::Occupied(existing_entry) = child_item
                        .as_mapping()
                        .unwrap()
                        .clone()
                        .entry("bash".into())
                    {
                        job.children.push(Box::new(Cmd {
                            args: existing_entry
                                .get()
                                .as_str()
                                .unwrap()
                                .split(' ')
                                .into_iter()
                                .map(|x| x.to_string())
                                .collect(),
                            work_dir: None,
                        }));
                    }
                }
                _ => {}
            }
        }

        println!("{}", job);
    }

    jobs
}

pub fn jobs_from_file(path: PathBuf) -> Vec<Job> {
    parse_jobs(read_yaml_file(path))
}
