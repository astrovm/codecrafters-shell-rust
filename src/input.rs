use std::io::{Error, Result, Write};

pub fn display_prompt() -> Result<()> {
    print!("$ ");

    // Make sure `$ ` appears before we wait for the user to type.
    std::io::stdout().flush()
}

// Return the typed text, None for Ctrl-D, or an error if reading failed.
pub fn read_input() -> Result<Option<String>> {
    // Give Linux an empty box that can hold up to 1,024 input bytes.
    let mut buffer = [0_u8; 1024];

    // Ask Linux to fill the box with bytes from standard input (the terminal).
    // SAFETY: the pointer leads to our box, and we give Linux its correct size.
    let bytes_read =
        unsafe { libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr().cast(), buffer.len()) };

    // Zero bytes means there is no more input. Ctrl-D does this at an empty prompt.
    if bytes_read == 0 {
        return Ok(None);
    }

    // -1 means Linux could not finish the read. Ctrl-C produces an
    // "interrupted" error, which main handles by showing a new prompt.
    if bytes_read == -1 {
        return Err(Error::last_os_error());
    }

    // Use only the part of the box that Linux filled, then turn it into text.
    // Invalid text bytes are replaced instead of crashing the shell.
    let input = String::from_utf8_lossy(&buffer[..bytes_read as usize]).into_owned();
    Ok(Some(input))
}

// Linux calls this tiny function when the user presses Ctrl-C.
extern "C" fn handle_sigint(_signal: libc::c_int) {
    // Because Linux uses this handler instead of its normal Ctrl-C behavior,
    // the shell stays open and the waiting `read` is interrupted.
    // Do nothing else here because most Rust operations are unsafe in a signal handler.
}

fn create_sigint_action() -> Result<libc::sigaction> {
    // Prepare an empty set of instructions that will describe what Ctrl-C does.
    // SAFETY: a zero-filled sigaction is valid on Linux.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };

    // Do not pause any extra signals while the Ctrl-C handler runs.
    // SAFETY: sa_mask is valid writable memory inside action.
    let result = unsafe { libc::sigemptyset(&mut action.sa_mask) };
    if result == -1 {
        return Err(Error::last_os_error());
    }

    // Tell Linux which function to call for Ctrl-C.
    action.sa_sigaction = handle_sigint as *const () as usize;

    // Use no extra options. This lets Ctrl-C stop a waiting `read`.
    action.sa_flags = 0;

    Ok(action)
}

pub fn install_sigint_handler() -> Result<()> {
    // Create the instructions Linux will use when Ctrl-C is pressed.
    let action = create_sigint_action()?;

    // Install these instructions for SIGINT, the signal produced by Ctrl-C.
    // The null pointer means we do not want the old instructions back.
    // SAFETY: `action` is fully initialized and stays alive during this call.
    let result = unsafe { libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) };
    if result == -1 {
        return Err(Error::last_os_error());
    }

    Ok(())
}
