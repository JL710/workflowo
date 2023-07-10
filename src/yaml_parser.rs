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
                    "bash" => match task_value {
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
                            panic!(
                                "bash task in {} has a problem with its definition",
                                root_key.as_str().unwrap()
                            );
                        }
                    },
                    "cmd" => {
                        job.children.push(Box::new(Cmd {
                            args: task_value
                                .as_str()
                                .unwrap()
                                .split(' ')
                                .into_iter()
                                .map(|x| x.to_string())
                                .collect(),
                            work_dir: None,
                        }));
                    }
                    _ => panic!("unrecognized task in {}", root_key.as_str().unwrap()),
                }
            }
        }

        println!("{}", job);
    }

    jobs
}

pub fn jobs_from_file(path: PathBuf) -> Vec<Job> {
    parse_jobs(read_yaml_file(path))
}
