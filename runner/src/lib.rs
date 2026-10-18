// Copyright 2026 Quantova Inc
// SPDX-License-Identifier: Apache-2.0 OR MIT

use qtv_account::{derive, derive_with_scheme};
use qtv_codec::{from_bytes, to_bytes};
use qtv_crypto::sha3;
use qtv_tx::{try_sign, Body, Call};

pub mod json;

use json::Json;

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn unhex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("a hex field has an odd length".to_string());
    }
    (0..text.len() / 2)
        .map(|i| {
            u8::from_str_radix(&text[2 * i..2 * i + 2], 16)
                .map_err(|_| "a hex field holds a non hex digit".to_string())
        })
        .collect()
}

fn same<T: PartialEq + std::fmt::Debug>(label: &str, got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!(
            "{label} recomputed {got:?} but the vector holds {want:?}"
        ))
    }
}

fn seed32(text: &str) -> Result<[u8; 32], String> {
    unhex(text)?
        .try_into()
        .map_err(|_| "a master seed is not 32 bytes".to_string())
}

fn load(text: &str) -> Result<Json, String> {
    json::parse(text)
}

fn decoded<T: qtv_codec::Decode>(label: &str, bytes: &[u8]) -> Result<T, String> {
    from_bytes::<T>(bytes).map_err(|_| format!("{label} bytes do not decode"))
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
    for text in CODEC {
        let vector = load(text)?;
        let kind = vector.str_of("kind")?;
        let want = vector.str_of("bytes")?;
        let wire = unhex(&want)?;
        let label = format!("codec.{}", vector.str_of("case")?);
        match kind.as_str() {
            "u32" => {
                let value = vector.u32_of("input")?;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, decoded::<u32>(&label, &wire)?, value)?;
            }
            "u64" => {
                let value = vector.u64_of("input")?;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, decoded::<u64>(&label, &wire)?, value)?;
            }
            "u128" => {
                let value = vector.num_of("input")?;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, decoded::<u128>(&label, &wire)?, value)?;
            }
            "bytes" => {
                let value = unhex(&vector.str_of("input")?)?;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, decoded::<Vec<u8>>(&label, &wire)?, value)?;
            }
            "option_some_u32" => {
                let value: Option<u32> = Some(vector.u32_of("input")?);
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, decoded::<Option<u32>>(&label, &wire)?, value)?;
            }
            "option_none_u32" => {
                let value: Option<u32> = None;
                same(&label, hex(&to_bytes(&value)), want.clone())?;
                same(&label, decoded::<Option<u32>>(&label, &wire)?, value)?;
            }
            other => return Err(format!("{label} names an unknown kind {other}")),
        }
        let mut longer = wire.clone();
        longer.push(0);
        let refused = match kind.as_str() {
            "u32" => from_bytes::<u32>(&longer).is_err(),
            "u64" => from_bytes::<u64>(&longer).is_err(),
            "u128" => from_bytes::<u128>(&longer).is_err(),
            "bytes" => from_bytes::<Vec<u8>>(&longer).is_err(),
            _ => from_bytes::<Option<u32>>(&longer).is_err(),
        };
        if !refused {
            return Err(format!("{label} accepts a trailing byte after the value"));
        }
    }
    Ok(())
}

pub fn check_address() -> Result<(), String> {
    let vector = load(include_str!("../../vectors/address.derivation.json"))?;
    let seed = seed32(&vector.str_of("master_seed")?)?;
    let account = derive(&seed, vector.u64_of("index")?);
    same(
        "address.scheme",
        u128::from(account.scheme()),
        vector.num_of("scheme")?,
    )?;
    same(
        "address.canonical",
        account.address(),
        vector.str_of("canonical")?,
    )
}

pub fn check_transaction() -> Result<(), String> {
    let vector = load(include_str!("../../vectors/transaction.transfer.json"))?;
    let seed = seed32(&vector.str_of("master_seed")?)?;
    let sender_account = derive(&seed, vector.u64_of("sender_index")?);
    let target = derive(&seed, vector.u64_of("target_index")?).address();
    let sender = sender_account.address();
    same(
        "transaction.sender",
        sender.clone(),
        vector.str_of("sender")?,
    )?;
    same(
        "transaction.target",
        target.clone(),
        vector.str_of("target")?,
    )?;

    let args = unhex(&vector.str_of("args")?)?;
    let call = Call::new(target, args);
    let body = Body::with_context(
        sender,
        vector.u64_of("nonce")?,
        vector.u64_of("meter_limit")?,
        vector.num_of("fee")?,
        call,
        0,
        qtv_tx::LOCAL_CHAIN_ID,
    );
    let body = match vector.str_of("kind")?.as_str() {
        "call" => body.calling(),
        "transfer" => body,
        other => return Err(format!("transaction names an unknown kind {other}")),
    };
    same(
        "transaction.body_bytes",
        hex(&to_bytes(&body)),
        vector.str_of("body_bytes")?,
    )?;

    let wrapper = try_sign(&sender_account, &body)
        .ok_or_else(|| "the vector addresses do not parse".to_string())?;
    same("transaction.tx_id", wrapper.id(), vector.str_of("tx_id")?)?;

    let bounded = vector.get("bounded")?;
    let deadline = body.clone().valid_until(bounded.u64_of("valid_until")?);
    let signed = try_sign(&sender_account, &deadline)
        .ok_or_else(|| "the bounded vector addresses do not parse".to_string())?;
    same(
        "transaction.bounded.tx_hex",
        hex(&to_bytes(&signed)),
        bounded.str_of("tx_hex")?,
    )?;
    same(
        "transaction.bounded.tx_id",
        signed.id(),
        bounded.str_of("tx_id")?,
    )?;
    if signed.id() == wrapper.id() {
        return Err("transaction.bounded the deadline does not change the signed bytes".into());
    }
    let other_chain = Body::with_context(
        body.sender().to_string(),
        body.nonce(),
        body.meter_limit(),
        body.fee(),
        body.call().clone(),
        0,
        qtv_tx::LOCAL_CHAIN_ID ^ 1,
    );
    let other_chain = match vector.str_of("kind")?.as_str() {
        "call" => other_chain.calling(),
        _ => other_chain,
    };
    if hex(&to_bytes(&other_chain)) == hex(&to_bytes(&body)) {
        return Err("transaction the chain id is not part of the signed body".into());
    }
    Ok(())
}

pub fn check_scheme_hash() -> Result<(), String> {
    let vector = load(include_str!(
        "../../vectors/scheme.hash_account_and_transfer.json"
    ))?;
    let seed = seed32(&vector.str_of("master_seed")?)?;
    let scheme = vector.u8_of("scheme")?;
    let sender_account = derive_with_scheme(&seed, scheme, vector.u64_of("sender_index")?)
        .ok_or_else(|| "the conformance scheme does not derive a sender".to_string())?;
    let target = derive_with_scheme(&seed, scheme, vector.u64_of("target_index")?)
        .ok_or_else(|| "the conformance scheme does not derive a target".to_string())?
        .address();
    let sender = sender_account.address();

    same(
        "scheme_hash.scheme",
        u128::from(sender_account.scheme()),
        vector.num_of("scheme")?,
    )?;
    same(
        "scheme_hash.canonical",
        sender_account.address(),
        vector.str_of("canonical")?,
    )?;
    same(
        "scheme_hash.sender",
        sender.clone(),
        vector.str_of("sender")?,
    )?;
    same(
        "scheme_hash.target",
        target.clone(),
        vector.str_of("target")?,
    )?;

    let args = unhex(&vector.str_of("args")?)?;
    let call = Call::new(target, args);
    let body = Body::with_context(
        sender,
        vector.u64_of("nonce")?,
        vector.u64_of("gas_limit")?,
        vector.num_of("fee")?,
        call,
        0,
        qtv_tx::LOCAL_CHAIN_ID,
    );
    same(
        "scheme_hash.body_bytes",
        hex(&to_bytes(&body)),
        vector.str_of("body_bytes")?,
    )?;

    let wrapper = try_sign(&sender_account, &body)
        .ok_or_else(|| "the vector addresses do not parse".to_string())?;
    same(
        "scheme_hash.signed_scheme",
        u128::from(wrapper.scheme()),
        vector.num_of("scheme")?,
    )?;
    same("scheme_hash.tx_id", wrapper.id(), vector.str_of("tx_id")?)
}

pub fn check_idfmt() -> Result<(), String> {
    let vector = load(include_str!("../../vectors/idfmt.families.json"))?;
    let input = unhex(&vector.str_of("input")?)?;
    let render = |label: &str, rendered: Result<String, qtv_idfmt::Error>| {
        rendered.map_err(|_| format!("{label} does not render"))
    };
    same(
        "idfmt.q1",
        render("idfmt.q1", qtv_idfmt::render_address(&input))?,
        vector.str_of("q1")?,
    )?;
    same(
        "idfmt.q2",
        render("idfmt.q2", qtv_idfmt::render_secret(&input))?,
        vector.str_of("q2")?,
    )?;
    same(
        "idfmt.qtx",
        render("idfmt.qtx", qtv_idfmt::render_tx(&input))?,
        vector.str_of("qtx")?,
    )?;
    same(
        "idfmt.qbk",
        render("idfmt.qbk", qtv_idfmt::render_block(&input))?,
        vector.str_of("qbk")?,
    )?;
    same(
        "idfmt.qst",
        render("idfmt.qst", qtv_idfmt::render_state(&input))?,
        vector.str_of("qst")?,
    )?;
    same(
        "idfmt.qcid",
        render("idfmt.qcid", qtv_idfmt::render_cid(&input))?,
        vector.str_of("qcid")?,
    )?;
    same(
        "idfmt.qpf",
        render("idfmt.qpf", qtv_idfmt::render_proof(&input))?,
        vector.str_of("qpf")?,
    )?;
    let q1 = vector.str_of("q1")?;
    let parsed =
        qtv_idfmt::parse_address(&q1).map_err(|_| "idfmt.q1 does not parse back".to_string())?;
    same("idfmt.q1.roundtrip", parsed, input.clone())?;
    let mut flipped: Vec<char> = q1.chars().collect();
    let last = flipped.len() - 1;
    flipped[last] = if flipped[last] == 'Q' { 'P' } else { 'Q' };
    let flipped: String = flipped.into_iter().collect();
    if qtv_idfmt::parse_address(&flipped).is_ok() {
        return Err("idfmt.q1 accepts an address with a broken checksum".into());
    }
    Ok(())
}

pub fn check_hostile() -> Result<(), String> {
    let hex_hash = load(include_str!(
        "../../vectors/hostile/idfmt.hex_hash_rejected.json"
    ))?;
    let offered = hex_hash.str_of("input")?;
    let refused = qtv_idfmt::parse_address(&offered).is_err()
        && qtv_idfmt::parse_secret(&offered).is_err()
        && qtv_idfmt::parse_tx(&offered).is_err()
        && qtv_idfmt::parse_block(&offered).is_err()
        && qtv_idfmt::parse_state(&offered).is_err()
        && qtv_idfmt::parse_cid(&offered).is_err()
        && qtv_idfmt::parse_proof(&offered).is_err();
    if !refused {
        return Err("hostile.hex_hash was parsed by an identifier family".into());
    }

    let floor = load(include_str!(
        "../../vectors/hostile/address.below_floor_rejected.json"
    ))?;
    let payload = unhex(&floor.str_of("payload")?)?;
    if payload.len() >= qtv_idfmt::KEY_FLOOR {
        return Err("hostile.floor payload reaches the floor".into());
    }
    if qtv_idfmt::render_address(&payload).is_ok() || qtv_idfmt::render_secret(&payload).is_ok() {
        return Err("hostile.floor payload rendered below the floor".into());
    }

    let vector = load(include_str!(
        "../../vectors/hostile/crypto.classical_not_sha3.json"
    ))?;
    let recomputed = hex(&sha3::sha3_256(b""));
    same(
        "hostile.sha3_empty",
        recomputed.clone(),
        vector.str_of("sha3_256_empty")?,
    )?;
    if recomputed == vector.str_of("classical256_empty")? {
        return Err("hostile.vector the stack digest equals the classical digest".into());
    }
    Ok(())
}

pub fn check_all() -> Result<(), String> {
    check_codec()?;
    check_address()?;
    check_transaction()?;
    check_scheme_hash()?;
    check_idfmt()?;
    check_hostile()
}
