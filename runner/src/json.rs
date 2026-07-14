//! A narrow reader for the frozen vector files. Each key is unique inside a

fn value_start(text: &str, key: &str) -> usize {
    let needle = format!("\"{key}\"");
    let bytes = text.as_bytes();
    let mut from = 0;
    loop {
        let rel = text[from..]
            .find(&needle)
            .unwrap_or_else(|| panic!("key {key} is absent"));
        let after_key = from + rel + needle.len();
        let mut at = after_key;
        while at < bytes.len() && matches!(bytes[at], b' ' | b'\t' | b'\n' | b'\r') {
            at += 1;
        }
        if at < bytes.len() && bytes[at] == b':' {
            at += 1;
            while at < bytes.len() && matches!(bytes[at], b' ' | b'\t' | b'\n' | b'\r') {
                at += 1;
            }
            return at;
        }
        from = after_key;
    }
}

pub fn str_field(text: &str, key: &str) -> String {
    let start = value_start(text, key);
    let bytes = text.as_bytes();
    assert_eq!(bytes[start], b'"', "field {key} is not a string");
    let open = start + 1;
    let mut end = open;
    while bytes[end] != b'"' {
        end += 1;
    }
    text[open..end].to_string()
}

pub fn num_field(text: &str, key: &str) -> u128 {
    let start = value_start(text, key);
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    text[start..end].parse().expect("digits parse")
}
