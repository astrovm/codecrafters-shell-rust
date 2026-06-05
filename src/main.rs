use std::io::{self, Write};

fn main() {
    loop {
        // Display the prompt before waiting for input
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read one line from standard input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        // Parse the input into a command and its arguments
        let input = input.trim();
        let (command, arguments) = input.split_once(' ').unwrap_or((input, ""));

        // Execute the matching built-in command or report an unknown command
        match command {
            "exit" => break,
            "echo" => println!("{}", arguments),
            "type" => println!("{}", type_command(arguments)),
            _ => println!("{}: command not found", command),
        }
    }
}

fn type_command(input: &str) -> String {
    // List the commands implemented directly by this shell
    let builtin_commands = ["exit", "echo", "type"];

    // Check whether the requested command is a shell built-in
    if builtin_commands.contains(&input) {
        format!("{} is a shell builtin", input)
    } else {
        format!("{}: not found", input)
    }
}
