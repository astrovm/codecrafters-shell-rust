use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

fn main() {
    loop {
        // Prompt for and read the next command.
        display_prompt();
        let input = read_input();

        // Parse quoted and unquoted text into separate arguments.
        let parsed = parse_arguments(&input);

        // Skip empty input; otherwise separate the command from its arguments.
        let Some((command, arguments)) = parsed.split_first() else {
            continue;
        };

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

fn parse_arguments(input: &str) -> Vec<String> {
    // Build arguments while tracking whether whitespace is inside single quotes.
    let mut arguments = Vec::new();
    let mut current_argument = String::new();
    let mut inside_single_quotes = false;

    for character in input.chars() {
        // Quotes control parsing but are not included in the argument.
        if character == '\'' {
            inside_single_quotes = !inside_single_quotes;
            continue;
        }

        // Unquoted whitespace ends the current argument and collapses repeats.
        if character.is_whitespace() && !inside_single_quotes {
            if !current_argument.is_empty() {
                arguments.push(std::mem::take(&mut current_argument));
            }
            continue;
        }

        current_argument.push(character);
    }

    // Save the final argument when the input does not end in whitespace.
    if !current_argument.is_empty() {
        arguments.push(current_argument);
    }

    arguments
}

fn command_dispatch(command: &str, arguments: &[String]) -> std::io::Result<()> {
    // Handle built-ins directly and delegate other commands for execution.
    match command {
        "echo" => {
            println!("{}", arguments.join(" "));
            Ok(())
        }
        "type" => {
            if let Some(arg) = arguments.first() {
                type_command(arg)
            } else {
                Ok(())
            }
        }
        "pwd" => pwd_command(),
        "cd" => cd_command(arguments.first()),
        _ => execute_command(command, arguments),
    }
}

fn type_command(input: &str) -> std::io::Result<()> {
    // Describe the command as a built-in, external executable, or missing.
    if BUILTIN_COMMANDS.contains(&input) {
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

fn cd_command(arguments: Option<&String>) -> std::io::Result<()> {
    // Use the home directory when no path or "~" is provided.
    let directory = arguments.map(String::as_str).unwrap_or("~");
    let expanded_path = if directory == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else {
        directory.to_string()
    };

    // Change the shell's working directory or report a missing path.
    if std::env::set_current_dir(&expanded_path).is_ok() {
        Ok(())
    } else {
        println!("cd: {}: No such file or directory", &expanded_path);
        Ok(())
    }
}

fn execute_command(command: &str, arguments: &[String]) -> std::io::Result<()> {
    // Run the executable with the arguments produced by the shell parser.
    if let Some(full_path) = find_executable_in_path(command) {
        Command::new(full_path)
            .arg0(command)
            .args(arguments)
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
