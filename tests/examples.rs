use std::process::Command;

fn cargo_in_project(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run cargo command")
}

#[test]
fn build_all_examples() {
    let output = cargo_in_project(&["build", "--examples", "--quiet"]);
    assert!(output.status.success(), "Example build failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn scoped_example_runs() {
    let output = cargo_in_project(&["run", "--example", "scoped_example", "--quiet"]);
    assert!(output.status.success(), "Scoped example failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starting scoped tracking example"));
}

#[test]
fn json_output_example_runs() {
    let output = cargo_in_project(&["run", "--example", "json_output_example", "--quiet"]);
    assert!(output.status.success(), "JSON output example failed: {}", String::from_utf8_lossy(&output.stderr));
}
