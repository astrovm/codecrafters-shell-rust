// This remembers how the next character should be understood.
enum ParserState {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
    EscapingUnquoted,
    EscapingDoubleQuoted,
}

struct ArgumentParser {
    // Arguments that are already finished.
    arguments: Vec<String>,

    // The argument we are building now.
    current_argument: String,

    // This lets us keep an empty quoted argument such as `''` or `""`.
    argument_started: bool,

    // This tells us whether special characters should keep their special meaning.
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

    // Quotes keep words together, but the quote characters are removed.
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
        // Decide what to do using both our current state and the next character.
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
                self.state = ParserState::EscapingUnquoted;
                self.argument_started = true;
            }
            (ParserState::DoubleQuoted, '\\') => {
                self.state = ParserState::EscapingDoubleQuoted;
            }
            (ParserState::SingleQuoted, '\'') => {
                self.state = ParserState::Unquoted;
            }
            (ParserState::DoubleQuoted, '"') => {
                self.state = ParserState::Unquoted;
            }
            (ParserState::EscapingUnquoted, _) => {
                self.state = ParserState::Unquoted;
                self.current_argument.push(character);
            }
            // Inside double quotes, `\"` and `\\` lose the backslash.
            // For every other character, keep the backslash.
            (ParserState::EscapingDoubleQuoted, '"' | '\\') => {
                self.state = ParserState::DoubleQuoted;
                self.current_argument.push(character);
            }
            (ParserState::EscapingDoubleQuoted, _) => {
                self.state = ParserState::DoubleQuoted;
                self.current_argument.push('\\');
                self.current_argument.push(character);
            }
            // Spaces finish an argument only when they are outside quotes.
            (ParserState::Unquoted, character) if character.is_whitespace() => {
                self.finish_argument();
            }
            // Everything else becomes part of the current argument.
            _ => {
                self.argument_started = true;
                self.current_argument.push(character);
            }
        }
    }

    fn finish_argument(&mut self) {
        if self.argument_started {
            // Move the finished argument into the list and leave behind an
            // empty String for the next argument.
            self.arguments
                .push(std::mem::take(&mut self.current_argument));
            self.argument_started = false;
        }
    }
}

pub fn parse_arguments(input: &str) -> Vec<String> {
    ArgumentParser::new().parse(input)
}
