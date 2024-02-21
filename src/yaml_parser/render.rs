use super::get_entry;
use anyhow::{bail, Context, Result};
use serde_yaml::{self, Value};
use std::collections::HashMap;
use std::io::Write;

/// resolves all tagged values recursively
pub fn render(_ids: &mut HashMap<String, Value>, value: &mut Value) -> Result<()> {
    match value {
        Value::Mapping(map) => {
            for map_value in map.values_mut() {
                render(_ids, map_value)?;
            }
        }
        Value::Sequence(seq) => {
            for item in seq {
                render(_ids, item)?;
            }
        }
        Value::Tagged(tagged) => {
            let mut new_value = match tagged.tag.to_string().as_str() {
                "!Input" => {
                    render_tag_input(_ids, &mut tagged.value, false).context("failed to resolve !Input")?
                }
                "!HiddenInput" => render_tag_input(_ids, &mut tagged.value, true)
                    .context("failed to resolve !HiddenInput")?,
                "!StrF" => {
                    render_tag_strf(_ids, &tagged.value).context("failed to resolve !StrF")?
                }
                "!Id" => render_tag_id(_ids, &mut tagged.value).context("failed to resolve !Id")?,
                _ => bail!(format!("{} is not a valid tag", tagged.tag)),
            };
            std::mem::swap(value, &mut new_value);
        }
        _ => {}
    }
    Ok(())
}

fn render_tag_strf(_ids: &mut HashMap<String, Value>, tag_value: &Value) -> Result<Value> {
    if !tag_value.is_sequence() {
        panic!("StringF needs to be a sequence of Strings",);
    }
    let mut formatted_string = String::new();
    for v in tag_value.as_sequence().unwrap().to_owned().iter_mut() {
        render(_ids, v)?;
        if !v.is_string() {
            panic!("StringF needs to be a sequence of strings",);
        }
        formatted_string += v.as_str().unwrap();
    }
    Ok(Value::String(formatted_string))
}

fn render_tag_input(_ids: &mut HashMap<String, Value>, tag_value: &mut Value, hidden: bool) -> Result<Value> {
    render(_ids, tag_value)?;
    // check if the input type is correct
    if !tag_value.is_string() && !tag_value.is_sequence() && !tag_value.is_mapping() {
        bail!("Input prompt is not a valid string, sequence or map");
    }
    let (prompt, default): (String, Option<String>) = match tag_value {
        Value::String(prompt) => (prompt.to_owned(), None),
        Value::Sequence(seq) => {
            // check if length is correct
            if seq.len() != 2 && seq.len() != 1 {
                bail!("!Input and !HiddenInput take 1 or 2 arguments but got {}", seq.len());
            }
            // check if prompt is string
            if !seq.get(0).unwrap().is_string() {
                bail!("Input prompt is not a valid string");
            }
            // check if prompt is string if given
            if seq.len() == 2 && !seq.get(1).unwrap().is_string() {
                bail!("Input default value is not a valid string");
            }
            // return
            (
                seq[0].as_str().unwrap().to_owned(),
                if seq.len() == 2 {
                    Some(seq[1].as_str().unwrap().to_owned())
                } else {
                    None
                },
            )
        }
        Value::Mapping(map) => {
            // get and check prompt
            let prompt = match get_entry(map, Value::String(String::from("prompt"))) {
                Some(value) => match value {
                    Value::String(text) => text,
                    _ => bail!("prompt is not of type string"),
                },
                None => bail!("prompt was not provided in !Input"),
            };
            // get and check default if given
            let default = match get_entry(map, Value::String(String::from("default"))) {
                Some(value) => match value {
                    Value::String(text) => Some(text),
                    _ => bail!("default is not of type string"),
                },
                _ => None,
            };
            // return
            (prompt, default)
        }
        _ => panic!("Input prompt is not a valid string, sequence or map"),
    };
    // print the prompt
    print!("{}", prompt);
    // get input
    let mut input = match hidden {
        true => {
            rpassword::prompt_password(prompt).context("hidden input failed (rpassword)")?
        },
        false => {
            std::io::stdout()
                .flush()
                .context("failed to flush stdout")?;
            let mut input_string = String::new();
            std::io::stdin()
                .read_line(&mut input_string)
                .context("failed to read line")?;
            input_string
        }
    };
    // remove linebraks
    while input.ends_with('\n') || input.ends_with('\r') {
        input.remove(input.len() - 1);
    }
    // replace with default if default and input is empty
    if default.is_some() && input.is_empty() {
        input = default.unwrap();
    }
    // return input
    Ok(Value::String(input))
}

fn render_tag_id(_ids: &mut HashMap<String, Value>, tag_value: &mut Value) -> Result<Value> {
    let id = match &tag_value {
        Value::Mapping(content_map) => match get_entry(content_map, "id".into()) {
            Some(id_value) => id_value
                .as_str()
                .context("!Id tag id is not of type string")?
                .to_owned(),
            _ => bail!("id key not given in id map"),
        },
        Value::Sequence(content_sequence) => {
            if content_sequence.is_empty() {
                bail!("!Id tag is missing its id");
            }
            content_sequence[0]
                .as_str()
                .context("invalid !Id tag id value. Value is not a string.")?
                .to_owned()
        }
        _ => bail!("!Id value needs to be a map or sequence!"),
    };

    let mut id_value = match &tag_value {
        Value::Mapping(content_map) => match get_entry(content_map, "value".into()) {
            Some(id_value) => id_value,
            _ => bail!("id key not given in id map"),
        },
        Value::Sequence(content_sequence) => {
            if content_sequence.len() < 2 {
                bail!("!Id tag is missing its value");
            }
            content_sequence[1].to_owned()
        }
        _ => bail!("!Id value needs to be a map or sequence!"),
    };

    if !_ids.contains_key(&id) {
        render(_ids, &mut id_value)?;
        _ids.insert(id.clone(), id_value);
    }

    Ok(_ids.get(&id).unwrap().to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn str_f_test() {
        use super::render;
        let content = "!StrF ['test', 'testa']";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value).unwrap();
        assert_eq!("testtesta", value.as_str().unwrap());
    }

    #[test]
    fn id_list_test() {
        use super::super::get_entry;
        use super::render;
        let content = "
        key1: !Id ['id', 'First Value']
        key2: !Id ['id', 'Second Value']
        ";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value).unwrap();
        // assert that at key2 the first value for the id `id` is used
        assert_eq!(
            "First Value",
            get_entry(value.as_mapping().unwrap(), "key2".into())
                .unwrap()
                .as_str()
                .unwrap()
        );
    }

    #[test]
    fn id_map_test() {
        use super::super::get_entry;
        use super::render;
        let content = "
        key1: !Id {id: 'id', value: 'First Value'}
        key2: !Id {id: 'id', value: 'Second Value'}
        ";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value).unwrap();
        // assert that at key2 the first value for the id `id` is used
        assert_eq!(
            "First Value",
            get_entry(value.as_mapping().unwrap(), "key2".into())
                .unwrap()
                .as_str()
                .unwrap()
        );
    }

    #[test]
    fn id_map_list_mixed_test1() {
        use super::super::get_entry;
        use super::render;

        let content = "
        key1: !Id ['id', 'First Value']
        key2: !Id {id: 'id', value: 'Second Value'}
        ";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value).unwrap();
        // assert that at key2 the first value for the id `id` is used
        assert_eq!(
            "First Value",
            get_entry(value.as_mapping().unwrap(), "key2".into())
                .unwrap()
                .as_str()
                .unwrap()
        );
    }

    #[test]
    fn id_map_list_mixed_test2() {
        use super::super::get_entry;
        use super::render;
        let content = "
        key1: !Id {id: 'id', value: 'First Value'}
        key2: !Id ['id', 'Second Value']
        ";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value).unwrap();
        // assert that at key2 the first value for the id `id` is used
        assert_eq!(
            "First Value",
            get_entry(value.as_mapping().unwrap(), "key2".into())
                .unwrap()
                .as_str()
                .unwrap()
        );
    }

    #[test]
    fn render_strf_nested_test() {
        use super::super::get_entry;
        use super::render;
        let content = "
        key1: !StrF ['test', 'testa']
        key2:
            - !StrF ['test', 'testa']
        key3:
            key3-1:
                - !StrF ['test', 'testa']
        ";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value).unwrap();

        assert!(get_entry(&value.as_mapping().unwrap(), "key1".into())
            .unwrap()
            .is_string());

        assert!(get_entry(&value.as_mapping().unwrap(), "key2".into())
            .unwrap()
            .as_sequence()
            .unwrap()
            .iter()
            .nth(0)
            .unwrap()
            .is_string());

        assert!(get_entry(
            get_entry(&value.as_mapping().unwrap(), "key3".into())
                .unwrap()
                .as_mapping()
                .unwrap(),
            "key3-1".into()
        )
        .unwrap()
        .as_sequence()
        .unwrap()
        .iter()
        .nth(0)
        .unwrap()
        .is_string())
    }
}
