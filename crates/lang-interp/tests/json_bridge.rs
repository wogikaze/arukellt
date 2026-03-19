use lang_interp::{Value, value_from_json, value_to_json, values_from_json_str};

#[test]
fn converts_tagged_object_into_variant_value() {
    let value = value_from_json(serde_json::json!({
        "tag": "Left",
        "fields": [7]
    }))
    .expect("variant value");

    assert_eq!(
        value,
        Value::Variant {
            name: "Left".to_owned(),
            fields: vec![Value::Int(7)],
        }
    );
}

#[test]
fn parses_argument_array_from_json_string() {
    let values =
        values_from_json_str(r#"[{"tag":"Right","fields":[2]}, true]"#).expect("argument values");

    assert_eq!(
        values,
        vec![
            Value::Variant {
                name: "Right".to_owned(),
                fields: vec![Value::Int(2)],
            },
            Value::Bool(true),
        ]
    );
}

#[test]
fn converts_variant_value_back_to_tagged_json() {
    let json = value_to_json(&Value::Variant {
        name: "Left".to_owned(),
        fields: vec![Value::Int(7)],
    });

    assert_eq!(
        json,
        serde_json::json!({
            "tag": "Left",
            "fields": [7]
        })
    );
}
