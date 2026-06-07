use std::ffi::OsString;
use std::io::Result;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

pub fn dispatch_command(command: &str, arguments: &[String]) -> Result<()> {
    // Run commands our shell knows itself. Search PATH for every other command.
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
    // If the user typed only `type`, there is no command name to check.
    let Some(argument) = argument else {
        return;
    };
    let argument = argument.as_str();

    // Check our shell's commands first, then look for a program in PATH.
    if BUILTIN_COMMANDS.contains(&argument) {
        println!("{} is a shell builtin", argument)
    } else if let Some(full_path) = find_executable_in_path(argument) {
        println!("{} is {}", argument, full_path.display())
    } else {
        println!("{}: not found", argument)
    }
}

fn pwd_command() -> Result<()> {
    // If Linux cannot tell us the current folder, return that error right away.
    let current_dir = std::env::current_dir()?;
    println!("{}", current_dir.display());
    Ok(())
}

fn cd_command(directory: Option<&String>) -> Result<()> {
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
        println!(
            "cd: {}: No such file or directory",
            expanded_path.to_string_lossy()
        );
        Ok(())
    }
}

fn execute_command(command: &str, arguments: &[String]) -> Result<()> {
    if let Some(full_path) = find_executable_in_path(command) {
        // Give the new program the command name the user typed.
        // Then start it and wait until it finishes.
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
