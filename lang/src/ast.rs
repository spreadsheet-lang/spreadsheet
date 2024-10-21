#![allow(unused)]
include!(concat!(env!("OUT_DIR"), "/ast.rs"));

#[cfg(test)]
mod tests {
    use cstree::interning::TokenInterner;
    use logos::Logos;

    use super::*;
    use crate::{
        grammar::Token,
        parser::{Parse, SyntaxNode},
    };

    #[track_caller]
    fn ok(src: &str) -> Parse {
        let parse = crate::parse(src);
        assert_eq!(parse.errors, vec![]);
        parse
    }

    #[test]
    // this just verifies we can do basic operations with the AST
    fn basic() {
        let input = "A1 = 5\n";
        let parse = dbg!(ok(input));
        let root = parse.red_tree();
        let root = root.green();
        assert_eq!(u32::from(root.text_len()), input.len() as u32);
        assert_eq!(root.children().len(), 1);
        let actual = root.children().next().unwrap();
        assert_eq!(actual.kind(), SyntaxKind::SOURCE_FILE.into());
        // TODO: add some more examples
    }
}
