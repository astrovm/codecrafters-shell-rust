use std::io::{Result, Write};

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
        return Err(std::io::Error::last_os_error());
    }

    // Use only the part of the box that Linux filled, then turn it into text.
    // Invalid text bytes are replaced instead of crashing the shell.
    let input = String::from_utf8_lossy(&buffer[..bytes_read as usize]).into_owned();
    Ok(Some(input))
}

// Linux calls this tiny function when the user presses Ctrl-C.
extern "C" fn handle_sigint(_signal: libc::c_int) {
    // Returning stops Ctrl-C from killing the shell and interrupts `read`.
    // Do nothing else here because most Rust operations are unsafe in a signal handler.
}

pub fn install_sigint_handler() -> Result<()> {
    // Prepare an empty set of instructions that will describe what Ctrl-C does.
    // `MaybeUninit` means the instructions are not ready to use yet.
    let mut action = std::mem::MaybeUninit::<libc::sigaction>::zeroed();

    // Do not pause any extra signals while the Ctrl-C handler runs.
    // SAFETY: `action` has enough writable space for these instructions.
    let result = unsafe { libc::sigemptyset(&mut (*action.as_mut_ptr()).sa_mask) };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    // The instructions are now initialized, so we can use normal field access.
    // SAFETY: the memory was zeroed and its signal list was initialized above.
    let mut action = unsafe { action.assume_init() };

    // Tell Linux which function to call for Ctrl-C.
    action.sa_sigaction = handle_sigint as *const () as usize;

    // Use no extra options. This lets Ctrl-C stop a waiting `read`.
    action.sa_flags = 0;

    // Install these instructions for SIGINT, the signal produced by Ctrl-C.
    // The null pointer means we do not want the old instructions back.
    // SAFETY: `action` is fully initialized and stays alive during this call.
    let result = unsafe { libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}
