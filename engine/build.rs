//! Handy build scripts.

use std::process::Command;

fn main() {
    // Compute the hash of the current Git commit.
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .expect("unable to get git version");
    let git_hash = String::from_utf8(output.stdout).expect("could not parse git version");
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
