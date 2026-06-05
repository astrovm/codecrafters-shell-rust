#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        // Display the shell prompt before waiting for input
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read one line of input from the user
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        // Trim the input and convert it to a string
        input = input.trim().to_string();

        // If the command is the builtin "exit", break the loop
        if input == "exit" {
            break;
        }

        // If the command starts with "echo ", print the rest of the input
        if input.starts_with("echo ") {
            println!("{}", input.trim_start_matches("echo "));
            continue;
        }

        // Report that the entered command is unknown
        println!("{}: command not found", input.trim())
    }
}
