mod commands;
mod input;
mod parser;

use commands::{ShellAction, dispatch_command};
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

        let parsed_command = parse_arguments(&input);

        // Take the first word as the command and keep the other words as its arguments.
        // If the user entered an empty line, show the next prompt.
        let Some((command, arguments)) = parsed_command.arguments.split_first() else {
            continue;
        };

        // Run the command, then either show another prompt or close the shell.
        let result = dispatch_command(command, arguments, parsed_command.redirections);
        match result {
            Ok(ShellAction::Continue) => continue,
            Ok(ShellAction::Exit) => break Ok(()),
            Err(error) => {
                eprintln!("{command}: {error}");
            }
        }
    }
}
