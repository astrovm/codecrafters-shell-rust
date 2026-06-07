mod commands;
mod input;
mod parser;

use commands::dispatch_command;
use input::{display_prompt, install_sigint_handler, read_input};
use parser::parse_arguments;
use std::io::Result;

fn main() -> Result<()> {
    // Tell Linux not to close our shell when the user presses Ctrl-C.
    install_sigint_handler()?;

    loop {
        display_prompt()?;

        // Normal text gives us Some(input).
        // Ctrl-D gives us None, so we close the shell.
        // Ctrl-C interrupts the read, so we start again with a new prompt.
        let input = match read_input() {
            Ok(Some(input)) => input,
            Ok(None) => {
                println!();
                break Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                println!();
                continue;
            }
            Err(error) => return Err(error),
        };

        let parsed_arguments = parse_arguments(&input);

        // Take the first word as the command and keep the other words as its arguments.
        // If the user entered an empty line, show the next prompt.
        let Some((command, arguments)) = parsed_arguments.split_first() else {
            continue;
        };

        if command == "exit" {
            // Stop the loop and finish the program successfully.
            break Ok(());
        }

        // Print an error only when running the command fails.
        if let Err(error) = dispatch_command(command, arguments) {
            eprintln!("{command}: {error}");
        }
    }
}
