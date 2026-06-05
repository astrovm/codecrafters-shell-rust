use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

fn main() -> std::io::Result<()> {
    loop {
        // Read one command line.
        display_prompt()?;
        let input = read_input()?;

        // Example: `echo 'hello world'` becomes ["echo", "hello world"].
        let parsed_arguments = parse_arguments(&input);

        // A blank line has no command. Otherwise, split the command name from
        // the arguments that follow it.
        let Some((command, arguments)) = parsed_arguments.split_first() else {
            continue;
        };

        // Stop reading commands when the user enters `exit`.
        if command == "exit" {
            break Ok(());
        }

        // Run a built-in command or an executable found in PATH.
        if let Err(error) = dispatch_command(command, arguments) {
            eprintln!("{command}: {error}");
        }
    }
}

fn display_prompt() -> std::io::Result<()> {
    // Show `$ ` before the program waits for input.
    print!("$ ");
    io::stdout().flush()
}

fn read_input() -> std::io::Result<String> {
    // Read everything the user types until Enter.
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn parse_arguments(input: &str) -> Vec<String> {
    // Convert a command line into separate arguments.
    //
    // `echo hello world`   -> ["echo", "hello", "world"]
    // `echo 'hello world'` -> ["echo", "hello world"]
    // `echo ''`            -> ["echo", ""]
    let mut arguments = Vec::new();

    // `''` creates an empty argument. This remembers that the quotes started
    // an argument even though `current_argument` contains no characters.
    let mut argument_started = false;

    // Build one argument here until unquoted whitespace ends it.
    let mut current_argument = String::new();

    // Inside single quotes, spaces belong to the argument instead of separating it.
    let mut inside_single_quotes = false;

    for character in input.chars() {
        // Enter or leave single quotes. Do not include the quote in the result.
        if character == '\'' {
            inside_single_quotes = !inside_single_quotes;
            argument_started = true;
            continue;
        }

        // Outside quotes, whitespace ends the current argument.
        // Extra whitespace is ignored instead of creating empty arguments.
        if character.is_whitespace() && !inside_single_quotes {
            if argument_started {
                argument_started = false;
                arguments.push(std::mem::take(&mut current_argument));
            }
            continue;
        }

        // Add normal characters, including spaces inside single quotes.
        argument_started = true;
        current_argument.push(character);
    }

    // Store the last argument because no trailing space is required.
    // For `echo ''`, argument_started is true and current_argument is empty.
    if argument_started {
        arguments.push(current_argument);
    }

    arguments
}

fn dispatch_command(command: &str, arguments: &[String]) -> std::io::Result<()> {
    // Send each command to its handler. Unknown names are treated as executables.
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
    // Print one space between arguments. Spaces inside quoted arguments remain.
    println!("{}", arguments.join(" "));
}

fn type_command(argument: Option<&String>) {
    // `type` without a command name prints nothing.
    let Some(argument) = argument else {
        return;
    };
    let argument = argument.as_str();

    // Check built-ins first, then executables in PATH.
    if BUILTIN_COMMANDS.contains(&argument) {
        println!("{} is a shell builtin", argument)
    } else if let Some(full_path) = find_executable_in_path(argument) {
        println!("{} is {}", argument, full_path.display())
    } else {
        println!("{}: not found", argument)
    }
}

fn pwd_command() -> std::io::Result<()> {
    // Print the shell's current directory.
    let current_dir = std::env::current_dir()?;
    println!("{}", current_dir.display());
    Ok(())
}

fn cd_command(directory: Option<&String>) -> std::io::Result<()> {
    // Both `cd` and `cd ~` use the home directory.
    let directory = directory.map(String::as_str).unwrap_or("~");
    let expanded_path = if directory == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else {
        directory.to_string()
    };

    // Change the directory used by later commands.
    if std::env::set_current_dir(&expanded_path).is_ok() {
        Ok(())
    } else {
        println!("cd: {}: No such file or directory", &expanded_path);
        Ok(())
    }
}

fn execute_command(command: &str, arguments: &[String]) -> std::io::Result<()> {
    // Find the executable, then run it with the parsed arguments.
    // Keep the typed command name as argument zero.
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
    // Search PATH directories in order and return the first matching executable.
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let full_path = dir.join(command);

        // Accept only regular files with at least one Unix execute bit.
        if let Ok(metadata) = full_path.metadata()
            && metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
        {
            return Some(full_path);
        }
    }

    None
}
