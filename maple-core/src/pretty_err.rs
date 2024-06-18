use std::fmt::format;

use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind, Source};

use thiserror::Error;

#[derive(Debug)]
pub struct DebugContext {
    file: String,
}

impl DebugContext {
    pub fn new(file_title: &str) -> Self {
        DebugContext {
            file: file_title.to_string(),
        }
    }

    pub fn expected_err(&self, expected: &str, found: &str, pos: usize) {

        let message = format!("Expected {}, found {}", expected, found);

        // Create a new color generator for the report
        let mut colors = ColorGenerator::new();

        // Generate a color for our error
        let error_color = colors.next();

        // Create a new report
        Report::build(ReportKind::Error, &self.file, pos)
            .with_message(message)
            .with_label(
                Label::new((&self.file, pos))
                    .with_message(message)
                    .with_color(error_color),
            )
            .finish()
            .print((&self.file, Source::from("")))
            .unwrap();

    }
}
