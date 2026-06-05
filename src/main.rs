use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    loop {
        // Prompt for and read the next command.
        display_prompt();
        let input = read_input();

        // Separate the command name from its arguments.
        let (command, arguments) = parse_input(&input);

        // Exit the shell loop when requested.
        if command == "exit" {
            break;
        }

        // Dispatch built-ins or try to run an external program.
        if let Err(error) = command_dispatch(command, arguments) {
            eprintln!("{command}: {error}");
        }
    }
}

fn display_prompt() {
    // Flush stdout so the prompt appears before input blocks.
    print!("$ ");
    io::stdout().flush().unwrap();
}

fn read_input() -> String {
    // Read a complete line from standard input.
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

fn parse_input(input: &str) -> (&str, &str) {
    // Use the whole input as the command when no arguments are present.
    let input = input.trim();
    input.split_once(' ').unwrap_or((input, ""))
}

fn command_dispatch(command: &str, arguments: &str) -> std::io::Result<()> {
    // Handle built-ins directly and delegate other commands for execution.
    match command {
        "echo" => {
            println!("{}", arguments);
            Ok(())
        }
        "type" => type_command(arguments),
        "pwd" => pwd_command(),
        _ => execute_command(command, arguments),
    }
}

fn type_command(input: &str) -> std::io::Result<()> {
    // Built-ins take precedence over executables with the same name.
    let builtin_commands = ["exit", "echo", "type", "pwd"];

    // Describe the command as a built-in, external executable, or missing.
    if builtin_commands.contains(&input) {
        println!("{} is a shell builtin", input)
    } else if let Some(full_path) = find_executable_in_path(input) {
        println!("{} is {}", input, full_path.display())
    } else {
        println!("{}: not found", input)
    }
    Ok(())
}

fn pwd_command() -> std::io::Result<()> {
    // Print the shell's current working directory.
    let current_dir = std::env::current_dir()?;
    println!("{}", current_dir.display());
    Ok(())
}

fn execute_command(command: &str, arguments: &str) -> std::io::Result<()> {
    // Run the executable with separate whitespace-delimited arguments.
    if let Some(full_path) = find_executable_in_path(command) {
        Command::new(full_path)
            .arg0(command)
            .args(arguments.split_whitespace())
            .status()?;
        Ok(())
    } else {
        println!("{command}: command not found");
        Ok(())
    }
}

fn find_executable_in_path(command: &str) -> Option<PathBuf> {
    // Search every directory listed in PATH.
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let full_path = dir.join(command);

        // Return the first regular file with an executable permission bit.
        if let Ok(metadata) = full_path.metadata()
            && metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
        {
            return Some(full_path);
        }
    }

    None
}
