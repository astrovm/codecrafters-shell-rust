// This remembers how the next character should be understood.
enum ParserState {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
    EscapingUnquoted,
    EscapingDoubleQuoted,
}

enum Token {
    // A command name, argument, or filename.
    Word(String),

    // An unquoted and unescaped `>` or `1>`.
    RedirectStdout,
}

struct ArgumentParser {
    // Words and operators that are already finished.
    tokens: Vec<Token>,

    // True when the current word has no quotes or backslash escapes.
    current_word_is_plain: bool,

    // The argument we are building now.
    current_argument: String,

    // This lets us keep an empty quoted argument such as `''` or `""`.
    argument_started: bool,

    // This tells us whether special characters should keep their special meaning.
    state: ParserState,
}

pub struct ParsedCommand {
    // The command name followed by its arguments.
    pub arguments: Vec<String>,

    // None means output stays on the terminal.
    pub stdout_file: Option<String>,
}

impl ArgumentParser {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current_word_is_plain: true,
            current_argument: String::new(),
            argument_started: false,
            state: ParserState::Unquoted,
        }
    }

    // Turn characters into word and redirection tokens.
    // Quotes keep words together, but the quote characters are removed.
    fn parse(mut self, input: &str) -> Vec<Token> {
        for character in input.chars() {
            self.handle_character(character);
        }

        self.finish_argument();

        self.tokens
    }

    fn handle_character(&mut self, character: char) {
        // Decide what to do using both our current state and the next character.
        match (&self.state, character) {
            (ParserState::Unquoted, '\'') => {
                self.state = ParserState::SingleQuoted;
                self.argument_started = true;
                self.current_word_is_plain = false;
            }
            (ParserState::Unquoted, '"') => {
                self.state = ParserState::DoubleQuoted;
                self.argument_started = true;
                self.current_word_is_plain = false;
            }
            (ParserState::Unquoted, '\\') => {
                self.state = ParserState::EscapingUnquoted;
                self.argument_started = true;
                self.current_word_is_plain = false;
            }
            (ParserState::Unquoted, '>') => {
                self.start_stdout_redirection();
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

    fn start_stdout_redirection(&mut self) {
        // In `1>`, the `1` names standard output and is not a command argument.
        if self.current_argument == "1" && self.current_word_is_plain {
            self.current_argument.clear();
            self.argument_started = false;
        } else {
            self.finish_argument();
        }
        self.tokens.push(Token::RedirectStdout);
    }

    fn finish_argument(&mut self) {
        if self.argument_started {
            // Save the finished word and leave an empty String for the next one.
            self.tokens
                .push(Token::Word(std::mem::take(&mut self.current_argument)));

            self.argument_started = false;
            self.current_word_is_plain = true;
        }
    }
}

pub fn parse_arguments(input: &str) -> ParsedCommand {
    // First find the words and operators, then work out what they mean.
    let tokens = ArgumentParser::new().parse(input);
    build_command(tokens)
}

fn build_command(tokens: Vec<Token>) -> ParsedCommand {
    // Normal words become command arguments. The word after `>` becomes
    // the output filename instead.
    let mut arguments = Vec::new();
    let mut stdout_file = None;
    let mut tokens = tokens.into_iter();

    while let Some(token) = tokens.next() {
        match token {
            Token::Word(arg) => arguments.push(arg),
            Token::RedirectStdout => {
                if let Some(Token::Word(path)) = tokens.next() {
                    stdout_file = Some(path);
                }
            }
        }
    }

    ParsedCommand {
        arguments,
        stdout_file,
    }
}
