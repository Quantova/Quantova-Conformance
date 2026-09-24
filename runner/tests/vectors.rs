// Copyright 2026 Quantova Inc
// SPDX-License-Identifier: Apache-2.0 OR MIT

use qtv_conformance_runner::{
    check_address, check_bridge, check_codec, check_hostile, check_idfmt, check_scheme_hash,
    check_transaction,
};

#[test]
fn codec() {
    check_codec().unwrap();
}

#[test]
fn address() {
    check_address().unwrap();
}

#[test]
fn transaction() {
    check_transaction().unwrap();
}

#[test]
fn scheme_hash() {
    check_scheme_hash().unwrap();
}

#[test]
fn idfmt() {
    check_idfmt().unwrap();
}

#[test]
fn hostile() {
    check_hostile().unwrap();
}

#[test]
fn bridge() {
    check_bridge().unwrap();
}

const UNCHECKED_HERE: &[(&str, &str)] = &[
    (
        "fee.in_band_at_several_prices.json",
        "no fee crate is a dependency of this runner",
    ),
    (
        "fee.stale_rate_clamps.json",
        "no fee crate is a dependency of this runner",
    ),
    (
        "vm.scratch_memory.json",
        "no machine crate is a dependency of this runner",
    ),
    (
        "hostile/fee.over_band_unenactable.json",
        "no fee crate is a dependency of this runner",
    ),
    (
        "hostile/scheme.unknown_id_unparseable.json",
        "no scheme registry crate is a dependency of this runner",
    ),
    (
        "hostile/consensus.non_mldsa_attestation_rejected.json",
        "no attestation crate is a dependency of this runner",
    ),
    (
        "hostile/governance.emergency_moves_funds.json",
        "mirrored from QONCORD and enforced by the constitutional gate there",
    ),
    (
        "hostile/governance.over_ceiling_mint.json",
        "mirrored from QONCORD and enforced by the constitutional gate there",
    ),
    (
        "hostile/governance.justice_touches_stake.json",
        "mirrored from QONCORD and enforced by the constitutional gate there",
    ),
];

#[test]
fn every_vector_is_either_checked_here_or_says_why_not() {
    let source = include_str!("../src/lib.rs");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the runner sits beside the vectors")
        .join("vectors");

    let mut files: Vec<String> = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("the vector directory is readable") {
            let path = entry.expect("a readable entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let rel = path
                    .strip_prefix(&root)
                    .expect("under the vector root")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push(rel);
            }
        }
    }
    assert!(!files.is_empty(), "the vector directory must not be empty");
    files.sort();

    let mut silent: Vec<String> = Vec::new();
    for file in &files {
        let read_by_runner = source.contains(file);
        let declared = UNCHECKED_HERE.iter().any(|(name, _)| name == file);
        if read_by_runner && declared {
            panic!("{file} is read by the runner and also listed as unchecked; the list is stale");
        }
        if !read_by_runner && !declared {
            silent.push(file.clone());
        }
    }
    assert!(
        silent.is_empty(),
        "these vectors are read by nothing and say nothing about it, so they look \
         like coverage and are not: {silent:?}. Give each one a checker, or name it \
         in UNCHECKED_HERE with the reason."
    );

    for (name, _) in UNCHECKED_HERE {
        assert!(
            files.iter().any(|f| f == name),
            "{name} is listed as unchecked but no such vector exists; the list is stale"
        );
    }
}
