use colored::*;


#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}


#[derive(Debug, Clone)]
pub struct ErrCtx {
    file_path: Option<String>,
    err_kind: Option<ErrorKind>,
    src: String,
    err_pos: Span,
}

impl ErrCtx {
    pub fn new(file_path: Option<&str>) -> Self {
        Self {
            file_path: file_path.map(|s| s.to_string()),
            err_kind: None,
            src: String::new(),
            err_pos: Span { start: 0, end: 0 },
        }
    }

    pub fn with_location(mut self, start: usize, end: usize) -> Self {
        self.err_pos = Span { start, end };
        self
    }

    pub fn with_src(mut self, src: String) -> Self {
        self.src = src;
        self
    }

    pub fn with_error_kind(mut self, err_kind: ErrorKind) -> Self {
        self.err_kind = Some(err_kind);
        self
    }

    pub fn display(&self) {
        if let Some(ref err_kind) = self.err_kind {
            if let Some(ref file_path) = self.file_path {
                println!("File: {}", file_path);
            }
            println!("Error: {}", err_kind.error().red());
            println!("Message: {}", err_kind.message().yellow());
            println!("Position: {}..{}", self.err_pos.start, self.err_pos.end);
            match self.src.get(self.err_pos.start..self.err_pos.end) {
                Some(fragment) => println!("Source: {}", fragment.green()),
                None => println!("Source: <invalid UTF-8 range>"),
            }
        } else {
            println!("No error information available.");
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
