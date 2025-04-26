use std::process::{Command, Stdio};

const BUN: &str = "/home/b/.bun/bin/bun";

fn bun(args: &[&str]) {
    let output = Command::new(BUN)
        .args(args)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .current_dir("personal-website")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stdout.lines() {
        println!("cargo::warning=o {line}");
    }
    for line in stderr.lines() {
        println!("cargo::warning=e {line}");
    }
}

fn main() {
    bun(&["i"]);
    bun(&["run", "build"]);
}
