use std::{io, ops::Range};

use annotate_snippets::{Renderer, Snippet};
use chumsky::span::Span;

#[derive(Debug)]
enum Error {
    Io(#[allow(dead_code)] io::Error),
    Parse,
}

impl From<io::Error> for Error {
    fn from(v: io::Error) -> Self {
        Self::Io(v)
    }
}

fn main() -> Result<(), Error> {
    let path = std::env::args().nth(1).unwrap();
    let input = std::fs::read_to_string(&path)?;
    let mut output = lang::parse(&input);
    // steal these errors so they don't get printed in the debug output
    let errs = std::mem::take(&mut output.errors);
    print!("{output:?}");
    if errs.is_empty() {
        Ok(())
    } else {
        for err in errs {
            let span = err.span();
            let () = span.context();
            let span = span.into_range();
            emit_error(
                "unexpected character",
                err.found()
                    .map(|c| format!("`{c}` was not expected here"))
                    .unwrap_or_else(|| "unexpected end of file".into()),
                span,
                &path,
                &input,
            );
        }
        Err(Error::Parse)
    }
}

fn emit_error(s: impl AsRef<str>, label: String, span: Range<usize>, file: &str, source: &str) {
    let mut msg = annotate_snippets::Level::Error.title(s.as_ref());

    let annotation = annotate_snippets::Level::Error.span(span).label(&label);
    let snippet = Snippet::source(&source)
        .fold(true)
        .origin(&file)
        .annotations(vec![annotation]);
    msg = msg.snippet(snippet);

    let renderer = if colored::control::SHOULD_COLORIZE.should_colorize() {
        Renderer::styled()
    } else {
        Renderer::plain()
    };
    eprintln!("{}", renderer.render(msg));
}
