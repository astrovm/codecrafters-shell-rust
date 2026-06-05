use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

fn main() -> std::io::Result<()> {
    loop {
        // Prompt for and read the next command.
        display_prompt()?;
        let input = read_input()?;

        // Parse quoted and unquoted text into separate arguments.
        let parsed_arguments = parse_arguments(&input);

        // Skip empty input; otherwise separate the command from its arguments.
        let Some((command, arguments)) = parsed_arguments.split_first() else {
            continue;
        };

        // Exit the shell loop when requested.
        if command == "exit" {
            break Ok(());
        }

        // Dispatch built-ins or try to run an external program.
        if let Err(error) = dispatch_command(command, arguments) {
            eprintln!("{command}: {error}");
        }
    }
}

fn display_prompt() -> std::io::Result<()> {
    // Flush stdout so the prompt appears before input blocks.
    print!("$ ");
    io::stdout().flush()
}

fn read_input() -> std::io::Result<String> {
    // Read a complete line from standard input.
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn parse_arguments(input: &str) -> Vec<String> {
    // Track whether an argument exists separately from its text so empty quoted
    // arguments are preserved.
    let mut arguments = Vec::new();
    let mut argument_started = false;
    let mut current_argument = String::new();
    let mut inside_single_quotes = false;

    for character in input.chars() {
        // A quote starts or ends a literal section without becoming output.
        if character == '\'' {
            inside_single_quotes = !inside_single_quotes;
            argument_started = true;
            continue;
        }

        // Only whitespace outside quotes separates arguments.
        if character.is_whitespace() && !inside_single_quotes {
            if argument_started {
                argument_started = false;
                arguments.push(std::mem::take(&mut current_argument));
            }
            continue;
        }

        argument_started = true;
        current_argument.push(character);
    }

    // Save the final argument, including an empty quoted argument.
    if argument_started {
        arguments.push(current_argument);
    }

    arguments
}

fn dispatch_command(command: &str, arguments: &[String]) -> std::io::Result<()> {
    // Handle built-ins directly and delegate other commands for execution.
    match command {
        "echo" => {
            echo_command(arguments);
            Ok(())
        }
        "type" => {
            type_command(arguments.first());
            Ok(())
        }
        "pwd" => pwd_command(),
        "cd" => cd_command(arguments.first()),
        _ => execute_command(command, arguments),
    }
}

fn echo_command(arguments: &[String]) {
    // Print parsed arguments separated by a single space.
    println!("{}", arguments.join(" "));
}

fn type_command(argument: Option<&String>) {
    // Do nothing when no command name is provided.
    let Some(argument) = argument else {
        return;
    };
    let argument = argument.as_str();

    // Describe the command as a built-in, external executable, or missing.
    if BUILTIN_COMMANDS.contains(&argument) {
        println!("{} is a shell builtin", argument)
    } else if let Some(full_path) = find_executable_in_path(argument) {
        println!("{} is {}", argument, full_path.display())
    } else {
        println!("{}: not found", argument)
    }
}

fn pwd_command() -> std::io::Result<()> {
    // Print the shell's current working directory.
    let current_dir = std::env::current_dir()?;
    println!("{}", current_dir.display());
    Ok(())
}

fn cd_command(directory: Option<&String>) -> std::io::Result<()> {
    // Use the home directory when no path or "~" is provided.
    let directory = directory.map(String::as_str).unwrap_or("~");
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
