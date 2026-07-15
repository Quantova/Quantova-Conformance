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
