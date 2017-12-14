extern crate solc;

fn main() {
    println!("CARGO_MANIFEST_DIR = {}", env!("CARGO_MANIFEST_DIR"));
    solc::compile(concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/"));
}
