use std::ffi::OsString;
use std::io::{Result, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

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

fn display_prompt() -> Result<()> {
    print!("$ ");

    // `print!` is buffered, so flush to show the prompt before waiting for input.
    std::io::stdout().flush()
}

// Return Some(line) for input, None for EOF, or an I/O error.
fn read_input() -> Result<Option<String>> {
    // Linux writes terminal input into this fixed-size byte buffer.
    let mut buffer = [0_u8; 1024];

    // SAFETY: `buffer` points to `buffer.len()` writable bytes for the duration
    // of the call. STDIN_FILENO identifies standard input.
    let bytes_read =
        unsafe { libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr().cast(), buffer.len()) };

    // A zero-byte read means EOF. At an empty terminal prompt, Ctrl-D causes it.
    if bytes_read == 0 {
        return Ok(None);
    }

    // C functions report failure with -1 and store details in `errno`.
    // `last_os_error` converts errno into a Rust I/O error. For Ctrl-C this is
    // ErrorKind::Interrupted, which main handles by restarting its loop.
    if bytes_read == -1 {
        return Err(std::io::Error::last_os_error());
    }

    // Only the first `bytes_read` bytes were initialized by Linux.
    // Lossy conversion replaces invalid UTF-8 instead of failing.
    let input = String::from_utf8_lossy(&buffer[..bytes_read as usize]).into_owned();
    Ok(Some(input))
}

// Linux calls this function when the terminal sends SIGINT for Ctrl-C.
extern "C" fn handle_sigint(_signal: libc::c_int) {
    // The handler only needs to return so `read` reports that it was interrupted.
    // Avoid Rust I/O, allocation, and locks because they are not signal-safe.
}

fn install_sigint_handler() -> Result<()> {
    // `MaybeUninit` gives the C API memory for a sigaction structure without
    // claiming that every field already contains a valid Rust value.
    let mut action = std::mem::MaybeUninit::<libc::sigaction>::zeroed();

    // `sa_mask` lists extra signals Linux should block while our handler runs.
    // SAFETY: `action` points to valid writable storage for a sigaction.
    let result = unsafe { libc::sigemptyset(&mut (*action.as_mut_ptr()).sa_mask) };

    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    // SAFETY: the structure was zeroed and its signal-set field was initialized.
    let mut action = unsafe { action.assume_init() };

    // Store the handler's address and use no optional flags. In particular,
    // leaving SA_RESTART disabled lets Ctrl-C interrupt `libc::read`.
    action.sa_sigaction = handle_sigint as *const () as usize;
    action.sa_flags = 0;

    // Register `action` as SIGINT's new behavior. A null third argument means
    // we do not need Linux to return the previously installed behavior.
    // SAFETY: `action` has the layout expected by libc and remains alive here.
    let result = unsafe { libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) };

    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

// A single enum prevents contradictory states such as being inside both quote types.
enum QuoteMode {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

struct ArgumentParser {
    // Completed arguments are moved here as separators are encountered.
    arguments: Vec<String>,

    // Characters for the argument currently being assembled.
    current_argument: String,

    // Distinguishes no argument from an empty quoted argument such as `''` or `""`.
    argument_started: bool,

    // Determines whether quotes and whitespace are syntax or literal text.
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
            // `take` moves out the completed String and leaves an empty String
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

fn dispatch_command(command: &str, arguments: &[String]) -> Result<()> {
    // Built-ins run inside this process. Other names are searched for in PATH.
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
    // `arguments.first()` supplied None when no command name was provided.
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

fn pwd_command() -> Result<()> {
    // `?` returns the I/O error immediately if the current directory is unknown.
    let current_dir = std::env::current_dir()?;
    println!("{}", current_dir.display());
    Ok(())
}

fn cd_command(directory: Option<&String>) -> Result<()> {
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

fn execute_command(command: &str, arguments: &[String]) -> Result<()> {
    if let Some(full_path) = find_executable_in_path(command) {
        // `arg0` preserves the name the user typed as the child's first process
        // argument. `status` starts the child and waits for it to finish.
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

    // PATH is an ordered list; the first executable match wins.
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
