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

        // Report that the entered command is unknown
        println!("{}: command not found", input.trim())
    }
}
