use std::process::Command;
fn main() {
    let output = Command::new("openapi-generator-cli")
        .args([
            "-g",
            "rust",
            "-o",
            "openapi_client",
            "-i",
            "openapi_spec/firecrest.yaml",
            "--package-name",
            "openapi_client",
        ])
        .output()
        .expect("Failed to execute command");
}
