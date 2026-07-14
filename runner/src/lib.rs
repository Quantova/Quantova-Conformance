//! Reproduces the frozen conformance vectors with the reference crates and

use qtv_codec::{from_bytes, to_bytes};

pub mod json;

use json::{num_field, str_field};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len() % 2 == 0, "hex length is even");
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex digit"))
        .collect()
}

fn same<T: PartialEq + std::fmt::Debug>(label: &str, got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{label} recomputed {got:?} but the vector holds {want:?}"))
    }
}

const CODEC: [&str; 6] = [
    include_str!("../../vectors/codec.u32.json"),
    include_str!("../../vectors/codec.u64.json"),
    include_str!("../../vectors/codec.u128.json"),
    include_str!("../../vectors/codec.bytes.json"),
    include_str!("../../vectors/codec.option_some.json"),
    include_str!("../../vectors/codec.option_none.json"),
];

pub fn check_codec() -> Result<(), String> {
    for vector in CODEC {
        let kind = str_field(vector, "kind");
        let want = str_field(vector, "bytes");
        let label = format!("codec.{}", str_field(vector, "case"));
        match kind.as_str() {
            "u32" => {
                let value = num_field(vector, "input") as u32;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, from_bytes::<u32>(&unhex(&want)).unwrap(), value)?;
            }
            "u64" => {
                let value = num_field(vector, "input") as u64;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, from_bytes::<u64>(&unhex(&want)).unwrap(), value)?;
            }
            "u128" => {
                let value = num_field(vector, "input");
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, from_bytes::<u128>(&unhex(&want)).unwrap(), value)?;
            }
            "bytes" => {
                let value = unhex(&str_field(vector, "input"));
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, from_bytes::<Vec<u8>>(&unhex(&want)).unwrap(), value)?;
            }
            "option_some_u32" => {
                let value: Option<u32> = Some(num_field(vector, "input") as u32);
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, from_bytes::<Option<u32>>(&unhex(&want)).unwrap(), value)?;
            }
            "option_none_u32" => {
                let value: Option<u32> = None;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, from_bytes::<Option<u32>>(&unhex(&want)).unwrap(), value)?;
            }
            other => return Err(format!("{label} names an unknown kind {other}")),
        }
    }
    Ok(())
}
