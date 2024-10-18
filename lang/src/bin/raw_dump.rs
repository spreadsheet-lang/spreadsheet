use std::io;

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
    let input = std::fs::read_to_string(path)?;
    let output = lang::parse(&input);
    if output.errors.is_empty() {
        dbg(output.red_tree(), 0);
        Ok(())
    } else {
        eprintln!("{:#?}", output.errors);
        Err(Error::Parse)
    }
}

fn dbg(node: lang::SyntaxNode, indent: usize) {
    println!("{}{:?}", " ".repeat(indent * 2), node);
    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Node(n) => dbg(n, indent + 1),
            rowan::NodeOrToken::Token(t) => println!("{}{:?}", " ".repeat((indent + 1) * 2), t),
        }
    }
}
