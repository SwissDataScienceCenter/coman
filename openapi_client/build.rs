use std::{
    io::{self, Write},
    process::Command,
};
fn main() {
    println!("cargo::rerun-if-changed=../openapi_spec/firecrest.yaml");
    let output = Command::new("openapi-generator-cli")
        .args([
            "-g",
            "rust",
            "-o",
            ".",
            "-i",
            "../openapi_spec/firecrest.yaml",
            "--package-name",
            "openapi_client",
        ])
        .output()
        .expect("Failed to execute command");
    io::stdout()
        .write_all(&output.stdout)
        .expect("couldn't write to stdout");
    io::stderr()
        .write_all(&output.stderr)
        .expect("couldn't write to stderr");
}
