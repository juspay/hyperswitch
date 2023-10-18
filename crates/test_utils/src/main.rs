use std::process::{exit, Command};

use test_utils::newman_runner;

fn main() {
    let mut runner: (Command, bool, String) = newman_runner::command_generate();

    // Execute the newman command
    let output = runner.0.spawn();
    let mut child = match output {
        Ok(child) => child,
        Err(err) => {
            eprintln!("Failed to execute command: {err}");
            exit(1);
        }
    };
    let status = child.wait();

    if runner.1 {
        let mut cmd = Command::new("git");
        let output = cmd
            .args([
                "checkout",
                "HEAD",
                "--",
                format!("{}/event.prerequest.js", runner.2).as_str(),
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let _ = String::from_utf8_lossy(&output.stdout);
                } else {
                    let _ = String::from_utf8_lossy(&output.stderr);
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    let exit_code = match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("Command executed successfully!");
                exit_status.code().unwrap_or(0)
            } else {
                eprintln!("Command failed with exit code: {:?}", exit_status.code());
                exit_status.code().unwrap_or(1)
            }
        }
        Err(err) => {
            eprintln!("Failed to wait for command execution: {err}");
            exit(1);
        }
    };

    exit(exit_code);
}
