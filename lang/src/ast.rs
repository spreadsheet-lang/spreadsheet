#![allow(unused)]
include!(concat!(env!("OUT_DIR"), "/ast.rs"));

#[cfg(test)]
mod tests {
    use crate::parser::SyntaxNode;
    use super::Root;

    fn assert_ok(src: &str) -> Root {
        let parse = crate::parse(src);
        assert!(parse.errors.is_empty());
        Root::cast(parse.red_tree()).unwrap()
    }

    #[test]
    fn basic() {
        let root = assert_ok("A1 = 5");
        root.statement();
    }
}
