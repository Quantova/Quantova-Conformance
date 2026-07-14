//! Reproduces the frozen conformance vectors with the reference crates and

use qtv_account::{derive, Tier};
use qtv_codec::{from_bytes, to_bytes};
use qtv_tx::{sign, Body, Call};

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

fn seed32(text: &str) -> [u8; 32] {
    let raw = unhex(text);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&raw);
    seed
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

pub fn check_address() -> Result<(), String> {
    let vector = include_str!("../../vectors/address.derivation.json");
    let seed = seed32(&str_field(vector, "master_seed"));
    let index = num_field(vector, "index") as u64;
    let account = derive(&seed, index);
    same("address.scheme", account.scheme() as u128, num_field(vector, "scheme"))?;
    same(
        "address.canonical",
        account.address_at(Tier::Canonical),
        str_field(vector, "canonical"),
    )?;
    same(
        "address.compact",
        account.address_at(Tier::Compact),
        str_field(vector, "compact"),
    )
}

pub fn check_transaction() -> Result<(), String> {
    let vector = include_str!("../../vectors/transaction.transfer.json");
    let seed = seed32(&str_field(vector, "master_seed"));
    let sender_account = derive(&seed, num_field(vector, "sender_index") as u64);
    let target = derive(&seed, num_field(vector, "target_index") as u64).address_at(Tier::Canonical);
    let sender = sender_account.address_at(Tier::Canonical);
    same("transaction.sender", sender.clone(), str_field(vector, "sender"))?;
    same("transaction.target", target.clone(), str_field(vector, "target"))?;

    let args = unhex(&str_field(vector, "args"));
    let call = Call::new(target, args);
    let body = Body::new(
        sender,
        num_field(vector, "nonce") as u64,
        num_field(vector, "gas_limit") as u64,
        num_field(vector, "fee"),
        call,
    );
    same("transaction.body_bytes", hex(&to_bytes(&body)), str_field(vector, "body_bytes"))?;

    let wrapper = sign(&sender_account, &body);
    same("transaction.tx_id", wrapper.id(), str_field(vector, "tx_id"))
}

pub fn check_idfmt() -> Result<(), String> {
    let vector = include_str!("../../vectors/idfmt.families.json");
    let input = unhex(&str_field(vector, "input"));
    same("idfmt.q1", qtv_idfmt::render_address(&input).unwrap(), str_field(vector, "q1"))?;
    same("idfmt.q2", qtv_idfmt::render_secret(&input).unwrap(), str_field(vector, "q2"))?;
    same("idfmt.qtx", qtv_idfmt::render_tx(&input).unwrap(), str_field(vector, "qtx"))?;
    same("idfmt.qbk", qtv_idfmt::render_block(&input).unwrap(), str_field(vector, "qbk"))?;
    same("idfmt.qst", qtv_idfmt::render_state(&input).unwrap(), str_field(vector, "qst"))?;
    same("idfmt.qcid", qtv_idfmt::render_cid(&input).unwrap(), str_field(vector, "qcid"))?;
    same("idfmt.qpf", qtv_idfmt::render_proof(&input).unwrap(), str_field(vector, "qpf"))
}
