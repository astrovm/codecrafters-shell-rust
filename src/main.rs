mod commands;
mod input;
mod parser;

use commands::dispatch_command;
use input::{display_prompt, install_sigint_handler, read_input};
use parser::parse_arguments;
use std::io::Result;

fn main() -> Result<()> {
    // Replace SIGINT's default behavior (terminating the shell) with our handler.
    install_sigint_handler()?;

    loop {
        display_prompt()?;

        // `read_input` uses Option to distinguish normal input from EOF:
        // - Some(input): the terminal supplied bytes.
        // - None: Ctrl-D produced EOF, so exit the shell.
        // - Interrupted: Ctrl-C stopped the read, so display a fresh prompt.
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

        // `split_first` separates the command from the remaining arguments.
        // An empty line returns None, so `let-else` skips to the next prompt.
        let Some((command, arguments)) = parsed_arguments.split_first() else {
            continue;
        };

        if command == "exit" {
            // The loop is the final expression, so this also returns from main.
            break Ok(());
        }

        // `if let` handles only failed commands; success needs no extra action.
        if let Err(error) = dispatch_command(command, arguments) {
            eprintln!("{command}: {error}");
        }
    }
}
