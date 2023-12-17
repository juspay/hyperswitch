use std::process::{exit, Command};

use test_utils::newman_runner;

fn main() {
    let mut runner = newman_runner::generate_newman_command();

    // Execute the newman command
    let output = runner.newman_command.spawn();
    let mut child = match output {
        Ok(child) => child,
        Err(err) => {
            eprintln!("Failed to execute command: {err}");
            exit(1);
        }
    };
    let status = child.wait();

    if runner.file_modified_flag {
        let git_status = Command::new("git")
            .args([
                "restore",
                format!("{}/event.prerequest.js", runner.collection_path).as_str(),
            ])
            .output();

        match git_status {
            Ok(output) => {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    println!("Git command executed successfully: {stdout_str}");
                } else {
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    eprintln!("Git command failed with error: {stderr_str}");
                }
            }
            Err(e) => {
                eprintln!("Error running Git: {e}");
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
