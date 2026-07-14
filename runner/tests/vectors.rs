use qtv_conformance_runner::{check_address, check_codec};

#[test]
fn codec() {
    check_codec().unwrap();
}

#[test]
fn address() {
    check_address().unwrap();
}
