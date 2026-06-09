// This remembers whether we are inside quotes.
enum QuoteMode {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

enum Token {
    // A command name, argument, or filename.
    Word(String),

    // Send normal output to the next filename.
    RedirectStdout,

    // Send error output to the next filename.
    RedirectStderr,
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

    // Characters follow different rules inside and outside quotes.
    quote_mode: QuoteMode,
}

pub struct ParsedCommand {
    // The command name followed by its arguments.
    pub arguments: Vec<String>,

    pub redirections: Redirections,
}

pub struct Redirections {
    // None means normal output stays on the terminal.
    pub stdout_file: Option<String>,

    // None means error output stays on the terminal.
    pub stderr_file: Option<String>,
}

impl Tokenizer {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current_word_is_plain: true,
            current_word: String::new(),
            word_started: false,
            quote_mode: QuoteMode::Unquoted,
        }
    }

    // Turn characters into word and redirection tokens.
    // Quotes keep words together, but the quote characters are removed.
    fn tokenize(mut self, input: &str) -> Vec<Token> {
        // A backslash can take the next character from this iterator.
        let mut characters = input.chars();
        while let Some(character) = characters.next() {
            self.handle_character(character, &mut characters);
        }

        self.finish_word();

        self.tokens
    }

    fn handle_character(&mut self, character: char, characters: &mut std::str::Chars<'_>) {
        // The same character can mean different things inside and outside quotes.
        match (&self.quote_mode, character) {
            (QuoteMode::Unquoted, '\'') => {
                self.quote_mode = QuoteMode::SingleQuoted;
                self.word_started = true;
                self.current_word_is_plain = false;
            }
            (QuoteMode::Unquoted, '"') => {
                self.quote_mode = QuoteMode::DoubleQuoted;
                self.word_started = true;
                self.current_word_is_plain = false;
            }
            // Outside quotes, a backslash keeps the next character but disappears.
            (QuoteMode::Unquoted, '\\') => {
                if let Some(next_character) = characters.next() {
                    self.current_word.push(next_character);
                }
                self.word_started = true;
                self.current_word_is_plain = false;
            }
            (QuoteMode::Unquoted, '>') => {
                self.start_redirection();
            }
            // Inside double quotes, `\"` and `\\` lose the backslash.
            // For every other character, keep the backslash.
            (QuoteMode::DoubleQuoted, '\\') => {
                if let Some(next_character) = characters.next() {
                    if next_character != '"' && next_character != '\\' {
                        self.current_word.push(character);
                    }
                    self.current_word.push(next_character);
                }
            }
            (QuoteMode::SingleQuoted, '\'') => {
                self.quote_mode = QuoteMode::Unquoted;
            }
            (QuoteMode::DoubleQuoted, '"') => {
                self.quote_mode = QuoteMode::Unquoted;
            }
            // Spaces finish a word only when they are outside quotes.
            (QuoteMode::Unquoted, character) if character.is_whitespace() => {
                self.finish_word();
            }
            // Everything else becomes part of the current word.
            _ => {
                self.word_started = true;
                self.current_word.push(character);
            }
        }
    }

    fn start_redirection(&mut self) {
        // A plain `1` means normal output, and a plain `2` means error output.
        let redirection = if self.current_word == "2" && self.current_word_is_plain {
            Token::RedirectStderr
        } else {
            Token::RedirectStdout
        };

        // Plain `1` and `2` belong to the operator, so do not save them as words.
        // Quoted or escaped versions are normal words and must be kept.
        if (self.current_word == "1" || self.current_word == "2") && self.current_word_is_plain {
            self.current_word.clear();
            self.word_started = false;
        } else {
            self.finish_word();
        }

        self.tokens.push(redirection);
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
    // Normal words become arguments. A word after a redirection becomes its filename.
    let mut arguments = Vec::new();
    let mut redirections = Redirections {
        stdout_file: None,
        stderr_file: None,
    };
    let mut tokens = tokens.into_iter();

    while let Some(token) = tokens.next() {
        match token {
            Token::Word(arg) => arguments.push(arg),
            Token::RedirectStdout => {
                if let Some(Token::Word(path)) = tokens.next() {
                    redirections.stdout_file = Some(path);
                }
            }
            Token::RedirectStderr => {
                if let Some(Token::Word(path)) = tokens.next() {
                    redirections.stderr_file = Some(path);
                }
            }
        }
    }

    ParsedCommand {
        arguments,
        redirections,
    }
}
