//! Reproduces the frozen conformance vectors with the reference crates and

use qtv_account::{derive, derive_with_scheme, Tier};
use qtv_codec::{from_bytes, to_bytes};
use qtv_crypto::sha3;
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

pub fn check_scheme_hash() -> Result<(), String> {
    let vector = include_str!("../../vectors/scheme.hash_account_and_transfer.json");
    let seed = seed32(&str_field(vector, "master_seed"));
    let scheme = num_field(vector, "scheme") as u8;
    let sender_index = num_field(vector, "sender_index") as u64;
    let target_index = num_field(vector, "target_index") as u64;
    let sender_account = derive_with_scheme(&seed, scheme, sender_index);
    let target = derive_with_scheme(&seed, scheme, target_index).address_at(Tier::Canonical);
    let sender = sender_account.address_at(Tier::Canonical);

    same(
        "scheme_hash.scheme",
        sender_account.scheme() as u128,
        num_field(vector, "scheme"),
    )?;
    same(
        "scheme_hash.canonical",
        sender_account.address_at(Tier::Canonical),
        str_field(vector, "canonical"),
    )?;
    same(
        "scheme_hash.compact",
        sender_account.address_at(Tier::Compact),
        str_field(vector, "compact"),
    )?;
    same(
        "scheme_hash.sender",
        sender.clone(),
        str_field(vector, "sender"),
    )?;
    same(
        "scheme_hash.target",
        target.clone(),
        str_field(vector, "target"),
    )?;

    let args = unhex(&str_field(vector, "args"));
    let call = Call::new(target, args);
    let body = Body::new(
        sender,
        num_field(vector, "nonce") as u64,
        num_field(vector, "gas_limit") as u64,
        num_field(vector, "fee"),
        call,
    );
    same(
        "scheme_hash.body_bytes",
        hex(&to_bytes(&body)),
        str_field(vector, "body_bytes"),
    )?;

    let wrapper = sign(&sender_account, &body);
    same(
        "scheme_hash.signed_scheme",
        wrapper.scheme() as u128,
        num_field(vector, "scheme"),
    )?;
    same(
        "scheme_hash.tx_id",
        wrapper.id(),
        str_field(vector, "tx_id"),
    )
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

pub fn check_hostile() -> Result<(), String> {
    let hex_hash = include_str!("../../vectors/hostile/idfmt.hex_hash_rejected.json");
    let offered = str_field(hex_hash, "input");
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

    let floor = include_str!("../../vectors/hostile/address.below_floor_rejected.json");
    let payload = unhex(&str_field(floor, "payload"));
    if payload.len() >= qtv_idfmt::KEY_FLOOR {
        return Err("hostile.floor payload reaches the floor".into());
    }
    if qtv_idfmt::render_address(&payload).is_ok() || qtv_idfmt::render_secret(&payload).is_ok() {
        return Err("hostile.floor payload rendered below the floor".into());
    }

    let keccak = include_str!("../../vectors/hostile/crypto.keccak_not_sha3.json");
    let recomputed = hex(&sha3::sha3_256(b""));
    same("hostile.sha3_empty", recomputed.clone(), str_field(keccak, "sha3_256_empty"))?;
    let classical = str_field(keccak, "keccak256_empty");
    if recomputed == classical {
        return Err("hostile.keccak the stack digest equals the classical digest".into());
    }
    Ok(())
}

pub fn check_bridge() -> Result<(), String> {
    let commingle =
        include_str!("../../vectors/hostile/bridge.same_symbol_commingling_rejected.json");
    let symbol = str_field(commingle, "symbol");
    let prefix = format!("q{symbol}.");
    if !str_field(commingle, "left_tag").starts_with(&prefix)
        || !str_field(commingle, "right_tag").starts_with(&prefix)
    {
        return Err("bridge.same_symbol a tag does not carry the shared symbol".into());
    }
    if str_field(commingle, "left_tag") == str_field(commingle, "right_tag") {
        return Err("bridge.same_symbol two origins share one tag".into());
    }
    if str_field(commingle, "left_origin") == str_field(commingle, "right_origin") {
        return Err("bridge.same_symbol the two origins are not distinct".into());
    }
    if str_field(commingle, "expected") != "rejected" {
        return Err("bridge.same_symbol commingling is not marked rejected".into());
    }

    let airlock = include_str!("../../vectors/hostile/bridge.non_airlock_artifact_unparseable.json");
    let artifact = str_field(airlock, "artifact_kind");
    if artifact == str_field(airlock, "airlock_form_one")
        || artifact == str_field(airlock, "airlock_form_two")
    {
        return Err("bridge.airlock the hostile artifact is actually an Airlock form".into());
    }
    if str_field(airlock, "expected") != "unparseable" {
        return Err("bridge.airlock the non Airlock artifact is not marked unparseable".into());
    }

    let stake = include_str!("../../vectors/hostile/bridge.bridged_asset_as_stake_rejected.json");
    if str_field(stake, "stake_asset_origin").is_empty() {
        return Err("bridge.stake the staked asset carries no origin tag".into());
    }
    if str_field(stake, "stake_asset_tag") == str_field(stake, "native_asset") {
        return Err("bridge.stake the staked asset is the native asset, not a bridged one".into());
    }
    if str_field(stake, "expected") != "rejected" {
        return Err("bridge.stake the bridged asset as stake is not marked rejected".into());
    }
    Ok(())
}

pub fn check_all() -> Result<(), String> {
    check_codec()?;
    check_address()?;
    check_transaction()?;
    check_scheme_hash()?;
    check_idfmt()?;
    check_hostile()?;
    check_bridge()
}
