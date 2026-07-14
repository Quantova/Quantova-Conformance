use qtv_conformance_runner::{check_address, check_codec};

fn main() {
    let steps: [(&str, fn() -> Result<(), String>); 2] =
        [("codec", check_codec), ("address", check_address)];

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
