use colored::*;

#[derive(Debug, Clone)]
pub struct DebugContext {
    file_path: String,
    err_kind: Option<ErrorKind>,
    src: String,
    line: usize,
    column: usize,
}

impl DebugContext {

    pub fn new(file_path: &str) -> Self {
        DebugContext {
            file_path: file_path.to_string(),
            err_kind: None,
            src: String::new(),
            line: 0,
            column: 0,
        }
    }

    pub fn is_err(&self) -> bool {
        self.err_kind.is_some()
    }
    
    pub fn set_source_code(&mut self, src: String) {
        self.src = src;
    }

    pub fn set_error(&mut self, kind: ErrorKind) {
        self.err_kind = Some(kind);
    }

    pub fn get_position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    pub fn set_position(&mut self, line: usize, column: usize) {
        self.line = line;
        self.column = column;
    }

    pub fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    pub fn display(&self) {
        if let Some(err_kind) = &self.err_kind {
            eprintln!("{} {}", "[ERROR]".red().bold(), err_kind.error().red());
            eprintln!("{} {}:{}:{}", "  -->".blue().bold(), self.file_path, self.line, self.column);
            eprintln!("{}", "  |".blue().bold());
            eprint!("{} {} ", self.line.to_string().blue().bold(), "|".blue().bold());
            eprintln!("{}", self.src.trim().dimmed());
            eprintln!("{}", "  |".blue().bold());
            eprint!("{}", "  =".blue().bold());
            eprintln!(" {}: {:?}", "help".bold(), err_kind.message());
            let maple_colour = Color::TrueColor { r: 236, g: 166, b: 124 };
            eprintln!("\n{} {}", "  Maple says".color(maple_colour), err_kind.name().bold().white());
        } else {
            eprintln!("{}", "Oh no! something went terribly wrong, but we don't know what!".red().italic());
            eprintln!("{}", "Please report this issue to the Maple project on GitHub.".red().italic());
        }
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    BrokenExpectations(String),
    UnpleasantSurprise(String),
    LonelyParenthesis(String),
    GrammarGoblin(String),
    NamelessNomad(String),
    AbruptAdieu(String),
}

impl ErrorKind {

    pub fn name(&self) -> String {
        match self {
            ErrorKind::BrokenExpectations(_) => "BrokenExpectations".to_string(),
            ErrorKind::UnpleasantSurprise(_) => "UnpleasantSurprise".to_string(),
            ErrorKind::LonelyParenthesis(_) => "LonelyParenthesis".to_string(),
            ErrorKind::GrammarGoblin(_) => "GrammarGoblin".to_string(),
            ErrorKind::NamelessNomad(_) => "NamelessNomad".to_string(),
            ErrorKind::AbruptAdieu(_) => "AbruptAdieu".to_string(),
        }
    }

    pub fn error(&self) -> String {
        match self {
            ErrorKind::BrokenExpectations(_) => format!("Expected token not found"),
            ErrorKind::UnpleasantSurprise(_) => format!("Unexpected token found"),
            ErrorKind::LonelyParenthesis(_) => format!("Unmatched parenthesis"),
            ErrorKind::GrammarGoblin(_) => format!("Syntax error"),
            ErrorKind::NamelessNomad(_) => format!("Unknown token found"),
            ErrorKind::AbruptAdieu(_) => format!("Unexpected end of file"),
        }
    }

    pub fn message(&self) -> String {
        match self {
            ErrorKind::BrokenExpectations(msg) => msg.clone(),
            ErrorKind::UnpleasantSurprise(msg) => msg.clone(),
            ErrorKind::LonelyParenthesis(msg) => msg.clone(),
            ErrorKind::GrammarGoblin(msg) => msg.clone(),
            ErrorKind::NamelessNomad(msg) => msg.clone(),
            ErrorKind::AbruptAdieu(msg) => msg.clone(),
        }
    }
}
