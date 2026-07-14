use qtv_conformance_runner::{check_address, check_codec, check_transaction};

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
