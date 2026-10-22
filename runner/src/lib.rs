// Copyright 2026 Quantova Inc
// SPDX-License-Identifier: Apache-2.0 OR MIT

use qtv_account::{derive, derive_with_scheme};
use qtv_codec::{from_bytes, to_bytes};
use qtv_crypto::sha3;
use qtv_tx::{Body, Call, Wrapper};

pub mod json;

use json::Json;

const VECTOR_GENESIS: [u8; 32] = [0u8; 32];

#[allow(deprecated)]
fn reference_sign(
    account: &qtv_account::Account,
    body: &Body,
    genesis: &[u8; 32],
) -> Option<Wrapper> {
    if qtv_idfmt::parse_address(body.sender()).is_err()
        || qtv_idfmt::parse_address(body.call().target()).is_err()
    {
        return None;
    }
    Some(qtv_tx::sign(account, body, genesis))
}

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
    if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("a hex field holds a non hex digit".to_string());
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

    let wrapper = reference_sign(&sender_account, &body, &VECTOR_GENESIS)
        .ok_or_else(|| "the vector addresses do not parse".to_string())?;
    same("transaction.tx_id", wrapper.id(), vector.str_of("tx_id")?)?;

    let bounded = vector.get("bounded")?;
    let deadline = body.clone().valid_until(bounded.u64_of("valid_until")?);
    let signed = reference_sign(&sender_account, &deadline, &VECTOR_GENESIS)
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
    same(
        "transaction.bounded.transfer_fee",
        u128::from(qtv_node::fee::FeeParams::devnet().transfer_fee()),
        bounded.num_of("transfer_fee")?,
    )?;
    let key = sender_account.public_key();
    if !qtv_tx::verify(&wrapper, key, &VECTOR_GENESIS)
        || !qtv_tx::verify(&signed, key, &VECTOR_GENESIS)
    {
        return Err("transaction the reference signatures do not verify".into());
    }
    if qtv_tx::verify(&signed, key, &[0x5au8; 32]) {
        return Err("transaction a signature verifies under another genesis".into());
    }
    let mut tampered = signed.signature().to_vec();
    tampered[0] ^= 1;
    let changed = Wrapper::new(signed.body().clone(), signed.scheme(), tampered);
    if qtv_tx::verify(&changed, key, &VECTOR_GENESIS) {
        return Err("transaction a changed signature still verifies".into());
    }
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

    let wrapper = reference_sign(&sender_account, &body, &VECTOR_GENESIS)
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
    check_fee()?;
    check_vm()?;
    check_scheme_registry()?;
    check_consensus_scheme()?;
    check_bridge()?;
    check_governance()?;
    check_hostile()
}

fn usd_micro_of(fee: u64, params: &qtv_node::fee::FeeParams) -> u128 {
    u128::from(fee) * params.rate_micro_usd_per_qtov / params.native_unit
}

fn fee_params(rate_micro_usd_per_qtov: u128) -> qtv_node::fee::FeeParams {
    let mut params = qtv_node::fee::FeeParams::devnet();
    params.rate_micro_usd_per_qtov = rate_micro_usd_per_qtov;
    params.native_unit = 1_000_000_000;
    params.max_fee_native = u64::MAX;
    params
}

pub fn check_fee() -> Result<(), String> {
    use qtv_node::fee::{MICRO_USD_CEILING, MICRO_USD_FLOOR};
    let band = load(include_str!(
        "../../vectors/fee.in_band_at_several_prices.json"
    ))?;
    same(
        "fee.band_min",
        MICRO_USD_FLOOR,
        band.num_of("fee_micro_usd_min")?,
    )?;
    same(
        "fee.band_max",
        MICRO_USD_CEILING,
        band.num_of("fee_micro_usd_max")?,
    )?;
    if band.str_of("expected")? != "every computed fee is within the band" {
        return Err("fee.in_band the vector changed its expectation".into());
    }
    let rates = match band.get("rates_micro_usd_per_native")? {
        Json::Array(items) => items
            .iter()
            .map(|item| match item {
                Json::Str(s) => s
                    .parse::<u128>()
                    .map_err(|_| "fee.in_band a rate is not a whole number".to_string()),
                _ => Err("fee.in_band a rate is not a string".to_string()),
            })
            .collect::<Result<Vec<u128>, String>>()?,
        _ => return Err("fee.in_band the rates are not a list".into()),
    };
    if rates.is_empty() {
        return Err("fee.in_band the vector names no rate".into());
    }
    for rate in rates {
        let params = fee_params(rate);
        let usd = usd_micro_of(params.transfer_fee(), &params);
        if !(MICRO_USD_FLOOR - 1..=MICRO_USD_CEILING).contains(&usd) {
            return Err(format!(
                "fee.in_band at rate {rate} the transfer fee is {usd} micro USD"
            ));
        }
    }

    let stale = load(include_str!("../../vectors/fee.stale_rate_clamps.json"))?;
    if stale.str_of("expected")? != "clamped within the band, never above" {
        return Err("fee.stale_rate_clamps the vector changed its expectation".into());
    }
    let params = fee_params(1_000_000);
    for asked in [0u128, 1, 499, 500, 750, 1000, 1001, 5_000, u128::MAX] {
        let usd = usd_micro_of(params.native_fee(asked), &params);
        if usd > MICRO_USD_CEILING || usd + 1 < MICRO_USD_FLOOR {
            return Err(format!(
                "fee.stale_rate_clamps {asked} micro USD became {usd}"
            ));
        }
    }

    let over = load(include_str!(
        "../../vectors/hostile/fee.over_band_unenactable.json"
    ))?;
    if over.str_of("expected")? != "unenactable" {
        return Err("fee.over_band the vector changed its expectation".into());
    }
    same(
        "fee.over_band.ceiling",
        MICRO_USD_CEILING,
        over.num_of("ceiling_micro_usd")?,
    )?;
    let asked = over.num_of("fee_micro_usd")?;
    if params.native_fee(asked) > params.ceiling_fee()
        || usd_micro_of(params.native_fee(asked), &params) > MICRO_USD_CEILING
    {
        return Err("fee.over_band a fee above the ceiling was produced".into());
    }
    Ok(())
}

fn vm_run(src: &str) -> Result<Result<qtv_vm::interp::Outcome, qtv_vm::interp::Fault>, String> {
    use qtv_vm::container::{selector, Container, Entry, StateAccess};
    let code = qtv_vm::asm::assemble(src).map_err(|e| format!("vm assemble {e:?}"))?;
    let sel = selector("probe()");
    let container = Container::new(
        code,
        vec![],
        vec![Entry {
            selector: sel,
            offset: 0,
            access: StateAccess::default(),
        }],
    );
    let interp = qtv_vm::interp::Interpreter::for_entry(&container, sel, 1_000_000)
        .map_err(|e| format!("vm entry {e:?}"))?;
    Ok(interp.run())
}

pub fn check_vm() -> Result<(), String> {
    use qtv_vm::interp::Fault;
    let vector = load(include_str!("../../vectors/vm.scratch_memory.json"))?;
    same(
        "vm.scratch_size",
        qtv_vm::state::MEM_BYTES as u128,
        vector.num_of("scratch_size_bytes")?,
    )?;
    let outcome = vm_run("LDI r1, 65528\nMLOAD r0, r1\nHALT")?
        .map_err(|e| format!("vm.zeroed faulted {e:?}"))?;
    if outcome.regs[0] != 0 {
        return Err("vm.zeroed the scratch memory is not zero at entry".into());
    }
    if vm_run("LDI r1, 65536\nMLOAD r0, r1\nHALT")? != Err(Fault::BadMemory) {
        return Err("vm.bounds a read past the scratch memory did not fault".into());
    }
    Ok(())
}

pub fn check_scheme_registry() -> Result<(), String> {
    let vector = load(include_str!(
        "../../vectors/hostile/scheme.unknown_id_unparseable.json"
    ))?;
    let id = u8::try_from(vector.num_of_hex("scheme_id")?).map_err(|_| "scheme id out of range")?;
    if qtv_tx::scheme_supported(id) || qtv_account::derive_with_scheme(&[7u8; 32], id, 0).is_some()
    {
        return Err("scheme.unknown an unregistered scheme is accepted".into());
    }
    Ok(())
}

pub fn check_consensus_scheme() -> Result<(), String> {
    let vector = load(include_str!(
        "../../vectors/hostile/consensus.non_mldsa_attestation_rejected.json"
    ))?;
    let id = u8::try_from(vector.num_of_hex("scheme_id")?).map_err(|_| "scheme id out of range")?;
    let account = qtv_account::derive_with_scheme(&[7u8; 32], id, 0)
        .ok_or("consensus.non_mldsa the scheme in the vector does not derive a key")?;
    if <qtv_crypto::ml_dsa::PublicKey>::try_from(account.public_key()).is_ok() {
        return Err("consensus.non_mldsa a non lattice key fits an attestation key".into());
    }
    Ok(())
}

pub fn check_bridge() -> Result<(), String> {
    use qtv_node::ledger::Ledger;
    let stake = load(include_str!(
        "../../vectors/hostile/bridge.bridged_asset_as_stake_rejected.json"
    ))?;
    if stake.str_of("expected")? != "rejected" {
        return Err("bridge.stake the vector changed its expectation".into());
    }
    let holder = derive(&[9u8; 32], 0);
    let address = holder.address();
    let id: [u8; 32] = qtv_idfmt::parse_address(&address)
        .map_err(|_| "bridge.stake address")?
        .try_into()
        .map_err(|_| "bridge.stake address length")?;
    let mut ledger = Ledger::new();
    if !ledger.mint_asset(&[0xe7u8; 32], &id, 1_000_000_000_000_000_000) {
        return Err("bridge.stake the bridged balance could not be minted".into());
    }
    if ledger.bond(&address, qtv_staking::MIN_STAKE, 0) {
        return Err("bridge.stake a holder of only a bridged asset bonded stake".into());
    }

    let mix = load(include_str!(
        "../../vectors/hostile/bridge.same_symbol_commingling_rejected.json"
    ))?;
    if mix.str_of("expected")? != "rejected"
        || mix.str_of("left_tag")? == mix.str_of("right_tag")?
    {
        return Err("bridge.commingling the vector changed its expectation".into());
    }
    let mut ledger = Ledger::new();
    let (eth, bsc) = ([0xe7u8; 32], [0xb5u8; 32]);
    ledger.mint_asset(&eth, &id, 500);
    if ledger.asset_balance_by_issuer(&bsc, &id) != 0
        || ledger.asset_balance_by_issuer(&eth, &id) != 500
    {
        return Err("bridge.commingling one symbol from two bridges shares a balance".into());
    }

    let foreign = load(include_str!(
        "../../vectors/hostile/bridge.non_airlock_artifact_unparseable.json"
    ))?;
    if foreign.str_of("expected")? != "unparseable" {
        return Err("bridge.airlock the vector changed its expectation".into());
    }
    let classical = vec![0x30u8, 0x44, 0x02, 0x20, 0xc1, 0xc1, 0xc1, 0xc1];
    if qlc_airlock::parse_ingress(&classical, qlc_stark::QUANTOVA_DEST_CHAIN_ID).is_ok() {
        return Err("bridge.airlock a classical signature parsed as an ingress".into());
    }
    Ok(())
}

pub fn check_governance() -> Result<(), String> {
    use qtv_constitution::{check_enactment, ConstitutionViolation, ProposedAction};
    let emergency = load(include_str!(
        "../../vectors/hostile/governance.emergency_moves_funds.json"
    ))?;
    let mint = load(include_str!(
        "../../vectors/hostile/governance.over_ceiling_mint.json"
    ))?;
    let seize = load(include_str!(
        "../../vectors/hostile/governance.recovery_from_unfrozen_stake.json"
    ))?;
    let a = mint.get("action")?;
    let over = check_enactment(&ProposedAction::Mint {
        amount: a.num_of("amount")?,
        epoch_minted: a.num_of("epoch_minted")?,
        epoch_ceiling: a.num_of("epoch_ceiling")?,
    });
    same(
        "governance.over_ceiling",
        over,
        Err(ConstitutionViolation::OverMintCeiling),
    )?;
    let moves = emergency.get("action")?.bool_of("moves_value")?;
    same(
        "governance.emergency",
        check_enactment(&ProposedAction::EmergencyPause { moves_value: moves }),
        Err(ConstitutionViolation::EmergencyMovesValue),
    )?;
    let s = seize.get("action")?;
    same(
        "governance.unfrozen_stake",
        check_enactment(&ProposedAction::JusticeSeize {
            targets_validator_stake: s.bool_of("targets_validator_stake")?,
            holder_frozen: s.bool_of("holder_frozen")?,
            within_bundle: s.bool_of("within_bundle")?,
        }),
        Err(ConstitutionViolation::StakeNotFrozen),
    )?;
    Ok(())
}
