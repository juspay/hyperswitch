use std::process::{exit, Command};

use test_utils::newman_runner;

fn main() {
    let mut newman_command: Command = newman_runner::command_generate();

    // Execute the newman command
    let output = newman_command.spawn();
    let mut child = match output {
        Ok(child) => child,
        Err(err) => {
            eprintln!("Failed to execute command: {err}");
            exit(1);
        }
    };
    let status = child.wait();

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
