use serde_json::{json, Value};

pub fn assert_expected_subset(actual: &Value, expected: &Value) -> Result<(), String> {
    match (actual, expected) {
        (Value::Object(actual_map), Value::Object(expected_map)) => {
            for (key, expected_value) in expected_map {
                let Some(actual_value) = actual_map.get(key) else {
                    return Err(format!("missing key `{key}` in actual state"));
                };
                assert_expected_subset(actual_value, expected_value)
                    .map_err(|message| format!("{key}: {message}"))?;
            }
            Ok(())
        }
        (Value::Array(actual_items), Value::Array(expected_items)) => {
            if actual_items.len() != expected_items.len() {
                return Err(format!(
                    "array length mismatch: expected {}, got {}",
                    expected_items.len(),
                    actual_items.len()
                ));
            }
            for (index, (actual_item, expected_item)) in
                actual_items.iter().zip(expected_items.iter()).enumerate()
            {
                assert_expected_subset(actual_item, expected_item)
                    .map_err(|message| format!("[{index}]: {message}"))?;
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!(
            "expected {}, got {}",
            json!(expected),
            json!(actual)
        )),
    }
}
