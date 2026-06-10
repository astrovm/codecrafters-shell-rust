use crate::parser::{Redirection, Redirections, WriteMode};
use std::ffi::OsString;
use std::fs::File;
use std::io::{Result, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

// Tell the main loop what to do after a command finishes.
pub enum ShellAction {
    Continue,
    Exit,
}

// Keep the two places where built-in commands can write together.
struct BuiltinStreams {
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}

impl BuiltinStreams {
    fn new(redirections: Redirections) -> Result<Self> {
        // Open the requested files, or use the terminal when there is no redirection.
        let stdout = create_stdout(redirections.stdout.as_ref())?;
        let stderr = create_stderr(redirections.stderr.as_ref())?;
        Ok(Self { stdout, stderr })
    }
}

pub fn dispatch_command(
    command: &str,
    arguments: &[String],
    redirections: Redirections,
) -> Result<ShellAction> {
    // Built-ins use our writers. External programs receive their files when started.
    if BUILTIN_COMMANDS.contains(&command) {
        let mut streams = BuiltinStreams::new(redirections)?;
        return execute_builtin(command, arguments, &mut streams);
    }

    execute_external_command(command, arguments, &redirections)
}

fn open_redirection(redirection: &Redirection) -> Result<File> {
    // Open the file in the way chosen by `>` or `>>`.
    match redirection.write_mode {
        WriteMode::Truncate => File::create(&redirection.file_path),
        WriteMode::Append => File::options()
            .create(true)
            .append(true)
            .open(&redirection.file_path),
    }
}

fn create_stdout(stdout_redirection: Option<&Redirection>) -> Result<Box<dyn Write>> {
    // Send normal output to a file when redirected, or to the terminal otherwise.
    match stdout_redirection {
        Some(redirection) => Ok(Box::new(open_redirection(redirection)?)),
        None => Ok(Box::new(std::io::stdout())),
    }
}

fn create_stderr(stderr_redirection: Option<&Redirection>) -> Result<Box<dyn Write>> {
    // Send errors to a file when redirected, or to the terminal otherwise.
    match stderr_redirection {
        Some(redirection) => Ok(Box::new(open_redirection(redirection)?)),
        None => Ok(Box::new(std::io::stderr())),
    }
}

fn execute_builtin(
    command: &str,
    arguments: &[String],
    streams: &mut BuiltinStreams,
) -> Result<ShellAction> {
    // Only `exit` closes the shell. Every other successful built-in continues.
    match command {
        "exit" => return Ok(ShellAction::Exit),
        "echo" => echo_command(arguments, &mut streams.stdout)?,
        "type" => type_command(arguments.first(), &mut streams.stdout)?,
        "pwd" => pwd_command(&mut streams.stdout)?,
        "cd" => cd_command(arguments.first(), &mut streams.stderr)?,
        _ => unreachable!(),
    }

    Ok(ShellAction::Continue)
}

fn echo_command(arguments: &[String], output: &mut dyn Write) -> Result<()> {
    writeln!(output, "{}", arguments.join(" "))
}

fn type_command(argument: Option<&String>, output: &mut dyn Write) -> Result<()> {
    // If the user typed only `type`, there is no command name to check.
    let Some(argument) = argument else {
        return Ok(());
    };
    let argument = argument.as_str();

    // Check our shell's commands first, then look for a program in PATH.
    if BUILTIN_COMMANDS.contains(&argument) {
        writeln!(output, "{} is a shell builtin", argument)
    } else if let Some(full_path) = find_executable_in_path(argument) {
        writeln!(output, "{} is {}", argument, full_path.display())
    } else {
        writeln!(output, "{}: not found", argument)
    }
}

fn pwd_command(output: &mut dyn Write) -> Result<()> {
    // If Linux cannot tell us the current folder, return that error right away.
    let current_dir = std::env::current_dir()?;
    writeln!(output, "{}", current_dir.display())
}

fn cd_command(directory: Option<&String>, err_output: &mut dyn Write) -> Result<()> {
    // Both `cd` and `cd ~` use the home directory.
    let directory = directory.map(String::as_str).unwrap_or("~");

    let expanded_path = if directory == "~" {
        // A Linux path is not always normal Rust text, so keep HOME as an OS string.
        std::env::var_os("HOME").unwrap_or_default()
    } else {
        OsString::from(directory)
    };

    if std::env::set_current_dir(&expanded_path).is_ok() {
        Ok(())
    } else {
        // Make a printable version of the path only for this error message.
        writeln!(
            err_output,
            "cd: {}: No such file or directory",
            expanded_path.to_string_lossy()
        )?;
        Ok(())
    }
}

fn execute_external_command(
    command: &str,
    arguments: &[String],
    redirections: &Redirections,
) -> Result<ShellAction> {
    if let Some(full_path) = find_executable_in_path(command) {
        let mut process = Command::new(full_path);

        // Give the new program the command name the user typed.
        process.arg0(command).args(arguments);

        // Send normal output to its file when the command redirects it.
        if let Some(redirection) = redirections.stdout.as_ref() {
            process.stdout(open_redirection(redirection)?);
        }

        // Send error output to its file when the command redirects it.
        if let Some(redirection) = redirections.stderr.as_ref() {
            process.stderr(open_redirection(redirection)?);
        }

        // Start the program and wait until it finishes.
        process.status()?;
        Ok(ShellAction::Continue)
    } else {
        let mut output = create_stderr(redirections.stderr.as_ref())?;
        writeln!(output, "{command}: command not found")?;
        Ok(ShellAction::Continue)
    }
}

fn find_executable_in_path(command: &str) -> Option<PathBuf> {
    // PATH may contain paths that are not normal Rust text, so use `var_os`.
    let path = std::env::var_os("PATH").unwrap_or_default();

    // Check PATH folders from left to right and use the first match.
    for dir in std::env::split_paths(&path) {
        let full_path = dir.join(command);

        // Use this path only when it is a file and Linux says someone may run it.
        // `0o111` checks the three "may run" permission bits.
        if let Ok(metadata) = full_path.metadata()
            && metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
        {
            return Some(full_path);
        }
    }

    None
}
