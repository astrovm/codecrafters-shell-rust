use std::io::{Result, Write};

pub fn display_prompt() -> Result<()> {
    print!("$ ");

    // `print!` is buffered, so flush to show the prompt before waiting for input.
    std::io::stdout().flush()
}

// Return Some(line) for input, None for EOF, or an I/O error.
pub fn read_input() -> Result<Option<String>> {
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

pub fn install_sigint_handler() -> Result<()> {
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
