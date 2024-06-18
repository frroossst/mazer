use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind, Source};


#[derive(Debug)]
pub struct DebugContext {
    file: String,
    errs: Vec<PrettyErrContext>,
}

impl DebugContext {
    pub fn new(file_title: &str) -> Self {
        DebugContext {
            file: file_title.to_string(),
            errs: Vec::new(),
        }
    }

    pub fn panic(&self) -> ! {
        // iterate over all reports and display them
        // finally panic
        for err in self.errs.iter() {
        }

        panic!()
    }

    pub fn push_new_error(&mut self, ctx: PrettyErrContext) {
    }
}

#[derive(Debug)]
pub struct PrettyErrContext {
    kind: PrettyErrKind,
    pos: (usize, usize),
    args: Vec<String>,
}

impl PrettyErrContext {
    pub fn new(kind: PrettyErrKind, pos: (usize, usize), args: Vec<String>) -> Self {
        PrettyErrContext {
            kind,
            pos,
            args,
        }
    }
}

#[derive(Debug)]
pub enum PrettyErrKind {
    ExpectedButNotFound,
}

