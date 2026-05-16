use serde_yaml::Value;

#[must_use]
pub fn string_value(text: &str) -> Value {
    Value::String(text.to_owned())
}
