use std::ffi::OsString;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

fn main() -> std::io::Result<()> {
    loop {
        // `?` returns early from `main` if either I/O operation fails.
        display_prompt()?;
        let input = read_input()?;

        let parsed_arguments = parse_arguments(&input);

        // Separate the command from its arguments. An empty line produces no
        // first item, so `let-else` skips to the next prompt.
        let Some((command, arguments)) = parsed_arguments.split_first() else {
            continue;
        };

        if command == "exit" {
            // The loop is the final expression in `main`, so breaking with
            // `Ok(())` finishes the program successfully.
            break Ok(());
        }

        // Only handle the error case here; successful commands need no action.
        if let Err(error) = dispatch_command(command, arguments) {
            eprintln!("{command}: {error}");
        }
    }
}

fn display_prompt() -> std::io::Result<()> {
    print!("$ ");
    // `print!` is buffered, so flush to show the prompt before waiting for input.
    io::stdout().flush()
}

fn read_input() -> std::io::Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

// A single enum prevents contradictory states such as being inside both quote types.
enum QuoteMode {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

struct ArgumentParser {
    arguments: Vec<String>,
    current_argument: String,
    // Distinguishes no argument from an empty quoted argument such as `''` or `""`.
    argument_started: bool,
    quote_mode: QuoteMode,
}

impl ArgumentParser {
    fn new() -> Self {
        Self {
            arguments: Vec::new(),
            current_argument: String::new(),
            argument_started: false,
            quote_mode: QuoteMode::Unquoted,
        }
    }

    // Quotes group text into one argument and are not included in the result.
    // `echo hello world`   -> ["echo", "hello", "world"]
    // `echo 'hello world'` -> ["echo", "hello world"]
    // `echo "hello world"` -> ["echo", "hello world"]
    // `echo ""`            -> ["echo", ""]
    fn parse(mut self, input: &str) -> Vec<String> {
        for character in input.chars() {
            self.handle_character(character);
        }

        self.finish_argument();
        self.arguments
    }

    fn handle_character(&mut self, character: char) {
        // Match the parser's current quote state together with the next character.
        match (&self.quote_mode, character) {
            (QuoteMode::Unquoted, '\'') => {
                self.quote_mode = QuoteMode::SingleQuoted;
                self.argument_started = true;
            }
            (QuoteMode::Unquoted, '"') => {
                self.quote_mode = QuoteMode::DoubleQuoted;
                self.argument_started = true;
            }
            (QuoteMode::SingleQuoted, '\'') => {
                self.quote_mode = QuoteMode::Unquoted;
                self.argument_started = true;
            }
            (QuoteMode::DoubleQuoted, '"') => {
                self.quote_mode = QuoteMode::Unquoted;
                self.argument_started = true;
            }
            // The `if` is a match guard: whitespace separates arguments only
            // when it appears outside quotes.
            (QuoteMode::Unquoted, character) if character.is_whitespace() => {
                self.finish_argument();
            }
            // Quotes that do not change the current mode, spaces inside quotes,
            // and ordinary characters are all literal argument content.
            _ => {
                self.argument_started = true;
                self.current_argument.push(character);
            }
        }
    }

    fn finish_argument(&mut self) {
        if self.argument_started {
            // `take` moves out the completed string and leaves an empty string
            // ready for the next argument.
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
    // `first` safely returns `None` when a command has no arguments.
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
    println!("{}", arguments.join(" "));
}

fn type_command(argument: Option<&String>) {
    // `let-else` returns from `type` when no command name was supplied.
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
    let current_dir = std::env::current_dir()?;
    println!("{}", current_dir.display());
    Ok(())
}

fn cd_command(directory: Option<&String>) -> std::io::Result<()> {
    // Both `cd` and `cd ~` use the home directory.
    let directory = directory.map(String::as_str).unwrap_or("~");
    let expanded_path = if directory == "~" {
        // Paths may contain non-UTF-8 bytes, so keep HOME as an OS string.
        std::env::var_os("HOME").unwrap_or_default()
    } else {
        OsString::from(directory)
    };

    if std::env::set_current_dir(&expanded_path).is_ok() {
        Ok(())
    } else {
        // Convert only for display; the original OS string remains unchanged.
        println!(
            "cd: {}: No such file or directory",
            expanded_path.to_string_lossy()
        );
        Ok(())
    }
}

fn execute_command(command: &str, arguments: &[String]) -> std::io::Result<()> {
    if let Some(full_path) = find_executable_in_path(command) {
        // `arg0` preserves the name the user typed. `status` starts the child
        // process and waits for it to finish.
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
    // `var_os` preserves PATH entries that are not valid UTF-8.
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let full_path = dir.join(command);

        // The chained conditions require readable metadata, a regular file,
        // and at least one Unix execute bit (`0o111`).
        if let Ok(metadata) = full_path.metadata()
            && metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
        {
            return Some(full_path);
        }
    }

    None
}
