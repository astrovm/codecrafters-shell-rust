// Exactly one parsing state is active for each input character.
enum ParserState {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
    BackslashEscaping,
}

struct ArgumentParser {
    // Completed arguments are moved here as separators are encountered.
    arguments: Vec<String>,

    // Characters for the argument currently being assembled.
    current_argument: String,

    // Distinguishes no argument from an empty quoted argument such as `''` or `""`.
    argument_started: bool,

    // Determines whether quotes, backslashes, and whitespace are syntax or literal text.
    state: ParserState,
}

impl ArgumentParser {
    fn new() -> Self {
        Self {
            arguments: Vec::new(),
            current_argument: String::new(),
            argument_started: false,
            state: ParserState::Unquoted,
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
        // Match the parser's current state together with the next character.
        match (&self.state, character) {
            (ParserState::Unquoted, '\'') => {
                self.state = ParserState::SingleQuoted;
                self.argument_started = true;
            }
            (ParserState::Unquoted, '"') => {
                self.state = ParserState::DoubleQuoted;
                self.argument_started = true;
            }
            (ParserState::Unquoted, '\\') => {
                self.state = ParserState::BackslashEscaping;
                self.argument_started = true;
            }
            (ParserState::SingleQuoted, '\'') => {
                self.state = ParserState::Unquoted;
                self.argument_started = true;
            }
            (ParserState::DoubleQuoted, '"') => {
                self.state = ParserState::Unquoted;
                self.argument_started = true;
            }
            (ParserState::BackslashEscaping, _) => {
                self.state = ParserState::Unquoted;
                self.argument_started = true;
                self.current_argument.push(character);
            }
            // The `if` is a match guard: whitespace separates arguments only
            // when it appears outside quotes.
            (ParserState::Unquoted, character) if character.is_whitespace() => {
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
