//! We use a concrete syntax tree (CST) represented by a rowan red-green tree
//! and parsed with chumsky.
//!
//! Unlike most ASTs, CSTs preserve comments and whitespace. Additionally,
//! Rowan particularly does not distinguish trivia from regular nodes, and uses
//! the same enum for both leaf nodes (tokens) and composite nodes (trees).

// separate mod to encapsulate the unsafety
mod syntax {
    #[repr(u16)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum SyntaxKind {
        // leaf nodes
        NEWLINE = 0,
        WHITESPACE,
        CELL,
        EQ,
        INT,

        // composite nodes
        ASSIGN,
        STATEMENT,
        // this MUST come last in the enum; we depend on it for memory safety
        ROOT,
    }

    impl From<SyntaxKind> for rowan::SyntaxKind {
        fn from(kind: SyntaxKind) -> Self {
            Self(kind as u16)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Lang {}
    impl rowan::Language for Lang {
        type Kind = SyntaxKind;
        fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
            assert!(raw.0 <= Self::Kind::ROOT as u16);
            // SAFETY: we just checked this is a valid variant.
            unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
        }
        fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
            kind.into()
        }
    }

    pub type SyntaxNode = rowan::SyntaxNode<Lang>;
    pub type SyntaxToken = rowan::SyntaxToken<Lang>;
    pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;
}

pub use syntax::*;

// now, we can do our actual parsing.
// use chumsky::extension::v1::{Ext, ExtParser};
use chumsky::prelude::*;
use rowan::{GreenNode, GreenNodeBuilder};
use text::whitespace;
use SyntaxKind::*;

type CSTError<'a> = Simple<'a, char>;
type CSTExtra<'a> = extra::Full<CSTError<'a>, GreenNodeBuilder<'a>, ()>;
trait CSTParser<'a, O = ()>: chumsky::Parser<'a, &'a str, O, CSTExtra<'a>> {}
impl<'a, O, T> CSTParser<'a, O> for T where T: chumsky::Parser<'a, &'a str, O, CSTExtra<'a>> {}

#[derive(Debug)]
pub struct Parse<'a> {
    pub root: GreenNode,
    pub errors: Vec<CSTError<'a>>,
}

impl Parse<'_> {
    /// Return a red tree based on the green tree we parsed, ignoring errors.
    ///
    /// Unlike a green tree, this has parent pointers, offsets, and identity semantics.
    /// It is meant to be used for temporary traversals, not for persistent storage.
    ///
    /// Note that this is still a homogeneous untyped tree. For example, our `ASSIGN` node
    /// does not encode that it has a PLACE and EXPRESSION node; we have to look up the number
    /// of child nodes at runtime.
    pub fn red_tree(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }
}

pub fn parse(text: &str) -> Parse {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(ROOT.into());
    let errors = parser().parse_with_state(text, &mut builder).into_errors();
    builder.finish_node();
    Parse {
        root: builder.finish(),
        errors,
    }
}

fn leaf<'a, O>(parser: impl CSTParser<'a, O>, kind: SyntaxKind) -> impl CSTParser<'a, ()> {
    parser.map_with(move |_, extra| {
        let slice = extra.slice();
        extra.state().token(kind.into(), slice);
    })
}

fn node<'a, O>(parser: impl CSTParser<'a, O>, kind: SyntaxKind) -> impl CSTParser<'a, ()> {
    empty::<_, CSTExtra<'a>>()
        .map_with(move |_, extra| {
            extra.state().start_node(kind.into());
        })
        .then(parser)
        .map_with(|_, extra| extra.state().finish_node())
}

macro_rules! leafs {
    ($(fn $fn:ident: $name:ident = $parser:expr);* $(;)? ) => {
        $( fn $fn<'a>() -> impl CSTParser<'a, ()> {
            leaf($parser, $name)
        } )*
    };
}

leafs! {
    fn eq: EQ = just('=');
    fn nl: NEWLINE = just('\n');
    fn int: INT = text::digits(10);
    // leaf(whitespace(), WHITESPACE),
}

#[rustfmt::skip]
fn parser<'a>() -> impl CSTParser<'a> {
    statement()
}

// AAA123
fn cell<'a>() -> impl CSTParser<'a, ()> {
    leaf(
        any()
            .filter(char::is_ascii_alphabetic)
            .repeated()
            .at_least(1)
            .then(any().filter(char::is_ascii_digit).repeated().at_least(1))
            .to_slice(),
        CELL,
    )
}

// A1 = 3
fn assign<'a>() -> impl CSTParser<'a> {
    node(cell().then(eq()).then(int()), ASSIGN)
}

fn statement<'a>() -> impl CSTParser<'a> {
    node(assign().then(nl()), STATEMENT)
}
