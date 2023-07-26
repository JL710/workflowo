use super::get_entry;
use serde_yaml::{self, Value};
use std::collections::HashMap;
use std::io::Write;

/// resolves all tagged values recursively
pub fn render(_ids: &mut HashMap<String, Value>, value: &mut Value) {
    match value {
        Value::Mapping(map) => {
            for map_value in map.values_mut() {
                render(_ids, map_value);
            }
        }
        Value::Sequence(seq) => {
            for item in seq {
                render(_ids, item);
            }
        }
        Value::Tagged(tagged) => {
            let mut new_value = match tagged.tag.to_string().as_str() {
                "!Input" => render_tag_input(_ids, &mut tagged.value),
                "!HiddenInput" => render_tag_hidden_input(_ids, &mut tagged.value),
                "!StrF" => render_tag_strf(_ids, &tagged.value),
                "!Id" => render_tag_id(_ids, &mut tagged.value),
                _ => panic!("{} is not a valid tag", tagged.tag),
            };
            std::mem::swap(value, &mut new_value);
        }
        _ => {}
    }
}

fn render_tag_strf(_ids: &mut HashMap<String, Value>, tag_value: &Value) -> Value {
    if !tag_value.is_sequence() {
        panic!("StringF needs to be a sequence of Strings",);
    }
    let mut formatted_string = String::new();
    for v in tag_value.as_sequence().unwrap().to_owned().iter_mut() {
        render(_ids, v);
        if !v.is_string() {
            panic!("StringF needs to be a sequence of strings",);
        }
        formatted_string += v.as_str().unwrap();
    }
    Value::String(formatted_string)
}

fn render_tag_hidden_input(_ids: &mut HashMap<String, Value>, tag_value: &mut Value) -> Value {
    render(_ids, tag_value);
    if !tag_value.is_string() {
        panic!("HiddenInput prompt is not a valid string")
    }
    Value::String(
        rpassword::prompt_password(tag_value.as_str().unwrap()).expect("rpassword input failed"),
    )
}

fn render_tag_input(_ids: &mut HashMap<String, Value>, tag_value: &mut Value) -> Value {
    render(_ids, tag_value);
    if !tag_value.is_string() {
        panic!("Input prompt is not a valid string")
    }
    print!("{}", tag_value.as_str().unwrap());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    while input.ends_with('\n') || input.ends_with('\r') {
        input.remove(input.len() - 1);
    }
    Value::String(input)
}

fn render_tag_id(_ids: &mut HashMap<String, Value>, tag_value: &mut Value) -> Value {
    let id = match &tag_value {
        Value::Mapping(content_map) => match get_entry(content_map, "id".into()) {
            Some(id_value) => id_value
                .as_str()
                .expect("!Id tag id is not of type string")
                .to_owned(),
            _ => panic!("id key not given in id map"),
        },
        Value::Sequence(content_sequence) => {
            if content_sequence.is_empty() {
                panic!("!Id tag is missing its id");
            }
            content_sequence[0]
                .as_str()
                .expect("invalid !Id tag id value. Value is not a string.")
                .to_owned()
        }
        _ => panic!("!Id value needs to be a map or sequence!"),
    };

    let mut id_value = match &tag_value {
        Value::Mapping(content_map) => match get_entry(content_map, "value".into()) {
            Some(id_value) => id_value,
            _ => panic!("id key not given in id map"),
        },
        Value::Sequence(content_sequence) => {
            if content_sequence.len() < 2 {
                panic!("!Id tag is missing its value");
            }
            content_sequence[1].to_owned()
        }
        _ => panic!("!Id value needs to be a map or sequence!"),
    };

    if !_ids.contains_key(&id) {
        render(_ids, &mut id_value);
        _ids.insert(id.clone(), id_value);
    }

    _ids.get(&id).unwrap().to_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn str_f_test() {
        use super::render;
        let content = "!StrF ['test', 'testa']";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value);
        assert_eq!("testtesta", value.as_str().unwrap());
    }

    #[test]
    fn id_test() {
        use super::super::get_entry;
        use super::render;
        let content = "
        key1: !Id ['id', 'First Value']
        key2: !Id ['id', 'Second Value']
        ";
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        render(&mut std::collections::HashMap::new(), &mut value);
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
        render(&mut std::collections::HashMap::new(), &mut value);

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
