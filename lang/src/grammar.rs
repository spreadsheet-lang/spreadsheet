use chumsky::prelude::*;

use crate::parser::{rowan_node as node, *};
use SyntaxKind::*;

macro_rules! leafs {
    ($(fn $fn:ident: $name:ident = $parser:expr);* $(;)? ) => {
        $( fn $fn<'a>() -> impl CSTParser<'a, ()> {
            leaf($name, $parser)
        } )*
    };
}

macro_rules! nodes {
    ($(fn $fn:ident: $name:ident = $parser:expr);* $(;)? ) => {
        $( fn $fn<'a>() -> impl CSTParser<'a, ()> {
            node($name, $parser)
        } )*
    };
}

fn ws<'a>() -> impl CSTParser<'a> {
    rowan_leaf(
        WHITESPACE,
        any::<_, CSTExtra<'a>>()
            .filter(|c: &char| *c != '\n' && c.is_whitespace())
            .repeated()
            .at_least(1)
            .or_not(),
    )
}

fn leaf<'a, O>(kind: SyntaxKind, parser: impl CSTParser<'a, O>) -> impl CSTParser<'a, ()> {
    ws().then_ignore(rowan_leaf(kind, parser))
}

leafs! {
    fn eq: EQ = just('=');
    fn nl: NEWLINE = just('\n');
    fn int: INT = text::digits(10);
    fn colon: COLON = just(':');
    fn dollar: DOLLAR = just('$');
    fn alias_tok: ALIAS_TOK = just("alias");
    fn enum_tok: ENUM_TOK = just("enum");
    fn ident: IDENT = chumsky::text::ident();
}

// AAA123
fn cell<'a>() -> impl CSTParser<'a, ()> {
    leaf(
        CELL,
        any()
            .filter(char::is_ascii_alphabetic)
            .repeated()
            .at_least(1)
            .then(any().filter(char::is_ascii_digit).repeated().at_least(1)),
    )
}

nodes! {
    // A1:A3
    fn cell_range: CELL_RANGE = cell().then(colon()).then(cell());
    // $foo
    fn alias_expr: ALIAS_EXPR = dollar().then(ident());
    fn place: PLACE = choice((cell_range(), alias_expr(), cell()));
    fn enum_expr: ENUM_EXPR = enum_tok().then(place());
    fn expr: EXPR = choice((enum_expr(), int(), place()));
    // A1 = 3
    fn assign: ASSIGN = place().then(eq()).then(expr());
    // alias foo = A1
    fn alias_stmt: ALIAS_STMT = alias_tok().then(ident()).then(eq()).then(place());
    fn statement: STATEMENT = choice((alias_stmt(), assign())).then(nl());
}

#[rustfmt::skip]
pub(crate) fn parser<'a>() -> impl CSTParser<'a> {
    choice((
        ws().then_ignore(nl()),
        statement(),
    ))
    .repeated().then_ignore(end())
}
