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

        // Example: `echo "hello world"` becomes ["echo", "hello world"].
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

struct ArgumentParser {
    // Arguments already completed at an unquoted space.
    arguments: Vec<String>,

    // The argument currently being assembled character by character.
    current_argument: String,

    // True after text or quotes begin an argument, including an empty `''` or `""`.
    argument_started: bool,

    // Quote modes decide whether spaces and the other quote character are literal.
    inside_single_quotes: bool,
    inside_double_quotes: bool,
}

impl ArgumentParser {
    // Start with no completed arguments, no current argument, and no active quote.
    fn new() -> Self {
        Self {
            arguments: Vec::new(),
            current_argument: String::new(),
            argument_started: false,
            inside_single_quotes: false,
            inside_double_quotes: false,
        }
    }

    // Convert a command line into separate arguments.
    //
    // `echo hello world`   -> ["echo", "hello", "world"]
    // `echo 'hello world'` -> ["echo", "hello world"]
    // `echo "hello world"` -> ["echo", "hello world"]
    // `echo ""`            -> ["echo", ""]
    fn parse(mut self, input: &str) -> Vec<String> {
        for character in input.chars() {
            self.handle_character(character);
        }

        // The input can end without a trailing space, so save what remains.
        self.finish_argument();
        self.arguments
    }

    // Decide whether one character changes quote state, separates arguments,
    // or belongs to the argument currently being built.
    fn handle_character(&mut self, character: char) {
        // A single quote opens or closes single-quote mode unless it appears
        // inside double quotes, where it is ordinary text.
        if character == '\'' && !self.inside_double_quotes {
            self.inside_single_quotes = !self.inside_single_quotes;
            self.argument_started = true;
            return;
        }

        // A double quote opens or closes double-quote mode unless it appears
        // inside single quotes, where it is ordinary text.
        if character == '"' && !self.inside_single_quotes {
            self.inside_double_quotes = !self.inside_double_quotes;
            self.argument_started = true;
            return;
        }

        // Outside quotes, whitespace ends the current argument.
        // Extra whitespace is ignored instead of creating empty arguments.
        if character.is_whitespace() && !self.inside_single_quotes && !self.inside_double_quotes {
            self.finish_argument();
            return;
        }

        // Add normal characters, including spaces inside either quote mode.
        self.argument_started = true;
        self.current_argument.push(character);
    }

    // Move a completed argument into the result and reset the temporary state.
    // `argument_started` preserves empty quoted arguments such as `''` and `""`.
    fn finish_argument(&mut self) {
        if self.argument_started {
            self.arguments
                .push(std::mem::take(&mut self.current_argument));
            self.argument_started = false;
        }
    }
}

fn parse_arguments(input: &str) -> Vec<String> {
    ArgumentParser::new().parse(input)
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
