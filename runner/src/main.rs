// Copyright 2026 Quantova Inc
// SPDX-License-Identifier: Apache-2.0 OR MIT

use qtv_conformance_runner::{
    check_address, check_codec, check_hostile, check_idfmt, check_transaction,
};

fn main() {
    let steps: [(&str, fn() -> Result<(), String>); 5] = [
        ("codec", check_codec),
        ("address", check_address),
        ("transaction", check_transaction),
        ("idfmt", check_idfmt),
        ("hostile", check_hostile),
    ];

    let mut failed = false;
    for (name, step) in steps {
        match step() {
            Ok(()) => println!("ok {name}"),
            Err(reason) => {
                println!("fail {name} {reason}");
                failed = true;
            }
        }
    }

    if failed {
        std::process::exit(1);
    }
}
