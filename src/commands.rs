use std::ffi::OsString;
use std::io::Result;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const BUILTIN_COMMANDS: [&str; 5] = ["exit", "echo", "type", "pwd", "cd"];

pub fn dispatch_command(command: &str, arguments: &[String]) -> Result<()> {
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
