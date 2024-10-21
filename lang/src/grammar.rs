use chumsky::prelude::*;
use logos::Logos;

use crate::parser::{rowan_node as node, *};
use SyntaxKind::*;

macro_rules! leafs {
    ($(fn $fn:ident: $name:ident = $token:ident);* $(;)? ) => {
        $( fn $fn<'a>() -> impl CSTParser<'a, ()> {
            leaf(SyntaxKind::$name, just(Token::$token))
        } )*
    };
}

macro_rules! nodes {
    ($($vis:vis fn $fn:ident: $name:ident = $parser:expr);* $(;)? ) => {
        $( $vis fn $fn<'a>() -> impl CSTParser<'a, ()> {
            node($name, $parser)
        } )*
    };
}

fn ws<'a>() -> impl CSTParser<'a, ()> {
    rowan_leaf(WHITESPACE, just(Token::Whitespace).or_not())
}

fn leaf<'a, O>(kind: SyntaxKind, parser: impl CSTParser<'a, O>) -> impl CSTParser<'a, ()> {
    ws().then_ignore(rowan_leaf(kind, parser))
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
pub enum Token {
    // needed for chumsky integration
    Error,

    // simple tokens
    #[token("=")]
    Eq,
    #[token("\n")]
    Nl,
    #[token("$")]
    Dollar,
    #[token(":")]
    Colon,
    #[token("alias")]
    Alias,
    #[token("enum")]
    Enum,

    // regex
    #[regex(r"[\p{White_space}--\n]+")]
    Whitespace,
    #[regex(r#"//[^\n]*\n"#)]
    Comment,
    // note that we don't have any string escapes
    #[regex(r#""[^"]*""#)]
    Str,
    #[regex(r#"\p{XID_Start}\p{XID_Continue}*"#)]
    Ident,
    #[regex(r"[a-zA-Z]+[0-9]+")]
    Cell,
    #[regex("[0-9]+")]
    Int,
}

// TODO: generate this with ungrammar?
leafs! {
    fn eq: EQ  = Eq;
    fn nl: NEWLINE = Nl;
    fn int: INT  = Int;
    fn colon: COLON  = Colon;
    fn dollar: DOLLAR  = Dollar;
    fn alias_tok: ALIAS_TOK  = Alias;
    fn enum_tok: ENUM_TOK  = Enum;
    fn ident: IDENT  = Ident;
    fn str: STR  = Str;
    fn comment: COMMENT  = Comment;
    fn cell: CELL  = Cell;
}

nodes! {
    // A1:A3
    fn cell_range: CELL_RANGE = cell().then(colon()).then(cell());
    // $foo
    fn alias_expr: ALIAS_EXPR = dollar().then(ident());
    fn place: PLACE = choice((cell_range(), alias_expr(), cell()));
    fn enum_expr: ENUM_EXPR = enum_tok().then(place());
    fn expr: EXPR = choice((enum_expr(), int(), str(), place()));
    // A1 = 3
    fn assign: ASSIGN = place().then(eq()).then(expr());
    // alias foo = A1
    fn alias_stmt: ALIAS_STMT = alias_tok().then(ident()).then(eq()).then(place());
    fn statement: STATEMENT = choice((alias_stmt(), assign())).then(nl());
    // NOTE: does not allow blank newlines between comments
    fn commented_statement: COMMENTED_STATEMENT = comment().repeated().then(statement());

    pub fn file: SOURCE_FILE =
        commented_statement()
        .padded_by(nl().repeated())
        .repeated();
}

#[cfg(test)]
mod test {
    use super::*;
    use Token::Nl;

    #[track_caller]
    fn ok<'a>(input: &'a str, parser: impl CSTParser<'a, ()>) -> Parse<'a> {
        let parse = parse_node(input, parser);
        assert!(
            parse.errors.is_empty(),
            "expected {input} to parse successfully, found invalid parse tree {parse:?} instead"
        );
        parse
    }

    #[track_caller]
    fn err<'a>(input: &'a str, parser: impl CSTParser<'a, ()>) -> Parse<'a> {
        let res = parse_node(input, parser);
        assert!(
            !res.errors.is_empty(),
            "expected {input} to have errors, found valid parse tree {res:?} instead"
        );
        res
    }

    #[test]
    fn lexer() {
        let lexer = Token::lexer("A1 = 5");
        assert_eq!(
            vec![
                Token::Cell,
                Token::Whitespace,
                Token::Eq,
                Token::Whitespace,
                Token::Int
            ],
            lexer.collect::<Result<Vec<_>, _>>().unwrap(),
        );
    }

    #[test]
    fn simple_str() {
        ok(r#""abc""#, str());
    }

    #[test]
    fn simple_comment() {
        ok("// foo\n", comment());
    }

    #[test]
    fn simple_cell() {
        ok("A1", cell());
        ok("A1 = 5", assign());
        ok("\n", nl());
        ok("A1 = 5\n", assign().then_ignore(just(Nl)));
        ok("A1 = 5\n", statement());
    }

    #[test]
    fn unknown_tokens() {
        err("```", file());
    }
}
