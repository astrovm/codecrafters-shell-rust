// A single enum prevents contradictory states such as being inside both quote types.
enum QuoteMode {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

struct ArgumentParser {
    // Completed arguments are moved here as separators are encountered.
    arguments: Vec<String>,

    // Characters for the argument currently being assembled.
    current_argument: String,

    // Distinguishes no argument from an empty quoted argument such as `''` or `""`.
    argument_started: bool,

    // Determines whether quotes and whitespace are syntax or literal text.
    quote_mode: QuoteMode,
}

impl ArgumentParser {
    fn new() -> Self {
        Self {
            arguments: Vec::new(),
            current_argument: String::new(),
            argument_started: false,
            quote_mode: QuoteMode::Unquoted,
        }
    }

    // Quotes group text into one argument and are not included in the result.
    // `echo hello world`   -> ["echo", "hello", "world"]
    // `echo 'hello world'` -> ["echo", "hello world"]
    // `echo "hello world"` -> ["echo", "hello world"]
    // `echo ""`            -> ["echo", ""]
    fn parse(mut self, input: &str) -> Vec<String> {
        for character in input.chars() {
            self.handle_character(character);
        }

        self.finish_argument();
        self.arguments
    }

    fn handle_character(&mut self, character: char) {
        // Match the parser's current quote state together with the next character.
        match (&self.quote_mode, character) {
            (QuoteMode::Unquoted, '\'') => {
                self.quote_mode = QuoteMode::SingleQuoted;
                self.argument_started = true;
            }
            (QuoteMode::Unquoted, '"') => {
                self.quote_mode = QuoteMode::DoubleQuoted;
                self.argument_started = true;
            }
            (QuoteMode::SingleQuoted, '\'') => {
                self.quote_mode = QuoteMode::Unquoted;
                self.argument_started = true;
            }
            (QuoteMode::DoubleQuoted, '"') => {
                self.quote_mode = QuoteMode::Unquoted;
                self.argument_started = true;
            }
            // The `if` is a match guard: whitespace separates arguments only
            // when it appears outside quotes.
            (QuoteMode::Unquoted, character) if character.is_whitespace() => {
                self.finish_argument();
            }
            // Quotes that do not change the current mode, spaces inside quotes,
            // and ordinary characters are all literal argument content.
            _ => {
                self.argument_started = true;
                self.current_argument.push(character);
            }
        }
    }

    fn finish_argument(&mut self) {
        if self.argument_started {
            // `take` moves out the completed String and leaves an empty String
            // ready for the next argument.
            self.arguments
                .push(std::mem::take(&mut self.current_argument));
            self.argument_started = false;
        }
    }
}

pub fn parse_arguments(input: &str) -> Vec<String> {
    ArgumentParser::new().parse(input)
}
