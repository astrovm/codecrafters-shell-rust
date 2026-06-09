use std::io::{Error, Result, Write};

// Restores the normal terminal settings when input reading ends.
struct TerminalModeGuard {
    original_settings: libc::termios,
}

impl Drop for TerminalModeGuard {
    fn drop(&mut self) {
        // SAFETY: these settings came from this terminal and remain valid.
        let _ =
            unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original_settings) };
    }
}

fn enable_raw_mode() -> Result<TerminalModeGuard> {
    // Ask Linux for the current terminal settings so we can restore them later.
    // SAFETY: Linux receives a valid place to store the settings.
    let mut original_settings: libc::termios = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut original_settings) };
    if result == -1 {
        return Err(Error::last_os_error());
    }

    // Receive each key immediately and let our shell print the typed characters.
    let mut raw_settings = original_settings;
    raw_settings.c_lflag &= !(libc::ICANON | libc::ECHO);

    // Apply the changed settings to standard input.
    // SAFETY: raw_settings is a valid copy of the terminal settings.
    let result = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw_settings) };
    if result == -1 {
        return Err(Error::last_os_error());
    }

    Ok(TerminalModeGuard { original_settings })
}

pub fn display_prompt() -> Result<()> {
    print!("$ ");

    // Make sure `$ ` appears before we wait for the user to type.
    std::io::stdout().flush()
}

fn redraw_input(input: &str, cursor: usize) -> Result<()> {
    // Return to the line start, print the current input, and erase old leftovers.
    print!("\r$ {input}\x1b[K");

    // Printing puts the cursor at the end, so move it back to its saved position.
    let distance = input.len() - cursor;
    if distance > 0 {
        print!("\x1b[{}D", distance);
    }

    std::io::stdout().flush()
}

fn read_byte() -> Result<Option<u8>> {
    // Read one key at a time.
    let mut buffer = [0_u8; 1];

    // SAFETY: the pointer leads to our one-byte box.
    let bytes_read =
        unsafe { libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr().cast(), buffer.len()) };

    // Zero bytes means there is no more input.
    if bytes_read == 0 {
        return Ok(None);
    }

    // -1 means Linux could not finish the read. Ctrl-C produces an
    // "interrupted" error, which main handles by showing a new prompt.
    if bytes_read == -1 {
        return Err(Error::last_os_error());
    }

    Ok(Some(buffer[0]))
}

// Return the typed text, None for Ctrl-D, or an error if reading failed.
pub fn read_input() -> Result<Option<String>> {
    // Normal terminal settings are restored automatically when this function ends.
    let _terminal_mode_guard = enable_raw_mode()?;

    let mut input = String::new();
    let mut cursor = 0;

    loop {
        let Some(byte) = read_byte()? else {
            return Ok(None);
        };

        match byte {
            // Enter finishes the line.
            10_u8 | 13_u8 => {
                println!();
                return Ok(Some(input));
            }
            // Ctrl-D closes the shell only when the line is empty.
            4_u8 => {
                if input.is_empty() {
                    return Ok(None);
                }
            }
            // Backspace removes the character just before the cursor.
            8_u8 | 127_u8 => {
                if cursor > 0 {
                    cursor -= 1;
                    input.remove(cursor);
                    redraw_input(&input, cursor)?;
                }
            }
            // Arrow keys start with Escape, followed by two more bytes.
            27_u8 => {
                let Some(second_byte) = read_byte()? else {
                    continue;
                };
                let Some(third_byte) = read_byte()? else {
                    continue;
                };

                match (second_byte, third_byte) {
                    // `ESC [ D` is the Left arrow.
                    (91_u8, 68_u8) if cursor > 0 => {
                        cursor -= 1;
                        redraw_input(&input, cursor)?;
                    }
                    // `ESC [ C` is the Right arrow.
                    (91_u8, 67_u8) if cursor < input.len() => {
                        cursor += 1;
                        redraw_input(&input, cursor)?;
                    }
                    // Ignore other arrows and moves past the ends of the input.
                    _ => {}
                }
            }
            // Insert normal ASCII keys wherever the cursor is.
            _ => {
                let character = byte as char;
                input.insert(cursor, character);
                cursor += 1;
                redraw_input(&input, cursor)?;
            }
        }
    }
}

// Linux calls this tiny function when the user presses Ctrl-C.
extern "C" fn handle_sigint(_signal: libc::c_int) {}

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
