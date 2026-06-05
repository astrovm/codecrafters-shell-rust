use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn main() {
    loop {
        // Display the prompt before waiting for input
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read one line from standard input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        // Parse the input into a command and its arguments
        let input = input.trim();
        let (command, arguments) = input.split_once(' ').unwrap_or((input, ""));

        // Execute the matching built-in command or report an unknown command
        match command {
            "exit" => break,
            "echo" => println!("{}", arguments),
            "type" => println!("{}", type_command(arguments)),
            _ => println!("{}: command not found", command),
        }
    }
}

fn type_command(input: &str) -> String {
    // List the commands implemented directly by this shell
    let builtin_commands = ["exit", "echo", "type"];

    // Check whether the requested command is a shell built-in
    if builtin_commands.contains(&input) {
        format!("{} is a shell builtin", input)
    } else if let Some(full_path) = find_executable_in_path(input) {
        format!("{} is {}", input, full_path.display())
    } else {
        format!("{}: not found", input)
    }
}

fn find_executable_in_path(command: &str) -> Option<PathBuf> {
    let path = std::env::var("PATH").unwrap_or_default();

    // Search each directory in PATH for the requested command
    for dir in std::env::split_paths(&path) {
        let full_path = dir.join(command);

        // Return the first regular file with an executable permission bit
        if let Ok(metadata) = full_path.metadata()
            && metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
        {
            return Some(full_path);
        }
    }
    None
}
