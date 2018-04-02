use std::process::Command;

fn main() {
    // make last git commit hash (`git rev-parse HEAD`)
    // available via `env!("GIT_HASH")` in sources
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
