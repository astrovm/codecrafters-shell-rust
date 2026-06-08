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

struct Tokenizer {
    // Words and operators that are already finished.
    tokens: Vec<Token>,

    // True when the current word has no quotes or backslash escapes.
    current_word_is_plain: bool,

    // The word we are building now.
    current_word: String,

    // This lets us keep an empty quoted word such as `''` or `""`.
    word_started: bool,

    // This tells us whether special characters should keep their special meaning.
    state: ParserState,
}

pub struct ParsedCommand {
    // The command name followed by its arguments.
    pub arguments: Vec<String>,

    // None means output stays on the terminal.
    pub stdout_file: Option<String>,
}

impl Tokenizer {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current_word_is_plain: true,
            current_word: String::new(),
            word_started: false,
            state: ParserState::Unquoted,
        }
    }

    // Turn characters into word and redirection tokens.
    // Quotes keep words together, but the quote characters are removed.
    fn tokenize(mut self, input: &str) -> Vec<Token> {
        for character in input.chars() {
            self.handle_character(character);
        }

        self.finish_word();

        self.tokens
    }

    fn handle_character(&mut self, character: char) {
        // Decide what to do using both our current state and the next character.
        match (&self.state, character) {
            (ParserState::Unquoted, '\'') => {
                self.state = ParserState::SingleQuoted;
                self.word_started = true;
                self.current_word_is_plain = false;
            }
            (ParserState::Unquoted, '"') => {
                self.state = ParserState::DoubleQuoted;
                self.word_started = true;
                self.current_word_is_plain = false;
            }
            (ParserState::Unquoted, '\\') => {
                self.state = ParserState::EscapingUnquoted;
                self.word_started = true;
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
                self.current_word.push(character);
            }
            // Inside double quotes, `\"` and `\\` lose the backslash.
            // For every other character, keep the backslash.
            (ParserState::EscapingDoubleQuoted, '"' | '\\') => {
                self.state = ParserState::DoubleQuoted;
                self.current_word.push(character);
            }
            (ParserState::EscapingDoubleQuoted, _) => {
                self.state = ParserState::DoubleQuoted;
                self.current_word.push('\\');
                self.current_word.push(character);
            }
            // Spaces finish a word only when they are outside quotes.
            (ParserState::Unquoted, character) if character.is_whitespace() => {
                self.finish_word();
            }
            // Everything else becomes part of the current word.
            _ => {
                self.word_started = true;
                self.current_word.push(character);
            }
        }
    }

    fn start_stdout_redirection(&mut self) {
        // In `1>`, the `1` names standard output and is not a command argument.
        if self.current_word == "1" && self.current_word_is_plain {
            self.current_word.clear();
            self.word_started = false;
        } else {
            self.finish_word();
        }
        self.tokens.push(Token::RedirectStdout);
    }

    fn finish_word(&mut self) {
        if self.word_started {
            // Save the finished word and leave an empty String for the next one.
            self.tokens
                .push(Token::Word(std::mem::take(&mut self.current_word)));

            self.word_started = false;
            self.current_word_is_plain = true;
        }
    }
}

pub fn parse_arguments(input: &str) -> ParsedCommand {
    // First find the words and operators, then work out what they mean.
    let tokens = Tokenizer::new().tokenize(input);
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
