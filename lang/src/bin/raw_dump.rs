use std::io;

fn main() -> Result<(), io::Error> {
    let path = std::env::args().nth(1).unwrap();
    let input = std::fs::read_to_string(path)?;
    let output = lang::parse(&input);
    assert!(output.errors.is_empty());
    dbg(output.red_tree(), 0);
    Ok(())
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
