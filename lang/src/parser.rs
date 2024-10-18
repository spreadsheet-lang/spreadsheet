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
    #[allow(non_camel_case_types)]
    pub enum SyntaxKind {
        // leaf nodes
        NEWLINE = 0,
        WHITESPACE,
        CELL,
        EQ,
        INT,
        COLON,
        // LEFT_BRACKET,
        // RIGHT_BRACKET,

        // composite nodes
        ASSIGN,
        STATEMENT,
        CELL_RANGE,
        // ARRAY_RANGE,
        PLACE,
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

use std::{fmt, marker::PhantomData};

pub use syntax::*;

// now, we can do our actual parsing.
// use chumsky::extension::v1::{Ext, ExtParser};
use chumsky::{
    extension::v1::{Ext, ExtParser},
    input::InputRef,
    prelude::*,
};
use rowan::{GreenNode, GreenNodeBuilder};
use SyntaxKind::*;

type CSTError<'a> = Simple<'a, char>;
type CSTExtra<'a> = extra::Full<CSTError<'a>, GreenNodeBuilder<'a>, ()>;
trait CSTParser<'a, O = ()>: chumsky::Parser<'a, &'a str, O, CSTExtra<'a>> {}
impl<'a, O, T> CSTParser<'a, O> for T where T: chumsky::Parser<'a, &'a str, O, CSTExtra<'a>> {}

pub struct Parse<'a> {
    pub root: GreenNode,
    pub errors: Vec<CSTError<'a>>,
}

impl<'a> fmt::Debug for Parse<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.errors.is_empty() {
            writeln!(f, "error: {:?}", self.errors)?;
        }
        Self::dbg(f, self.red_tree(), 0)
    }
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

    fn dbg(f: &mut fmt::Formatter, node: SyntaxNode, indent: usize) -> fmt::Result {
        writeln!(f, "{}{:?}", " ".repeat(indent * 2), node)?;
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(n) => Self::dbg(f, n, indent + 1)?,
                rowan::NodeOrToken::Token(t) => {
                    writeln!(f, "{}{:?}", " ".repeat((indent + 1) * 2), t)?;
                }
            }
        }
        Ok(())
    }
}

pub fn parse(text: &str) -> Parse {
    let mut builder = GreenNodeBuilder::new();
    // we don't put this in parser() to ensure rowan never panics even on horribly invalid programs
    builder.start_node(ROOT.into());
    let errors = parser().parse_with_state(text, &mut builder).into_errors();
    builder.finish_node();
    // dbg!(&errors, &builder);
    Parse {
        root: builder.finish(),
        errors,
    }
}

fn ws<'a>() -> impl CSTParser<'a> {
    any::<_, CSTExtra<'a>>()
        .filter(|c: &char| *c != '\n' && c.is_whitespace())
        .repeated()
        .map_with(|_, extra| {
            let slice: &str = extra.slice();
            if !slice.is_empty() {
                // println!("ws '{slice}'");
                extra.state().token(WHITESPACE.into(), slice);
            }
        })
}

fn leaf<'a, O>(parser: impl CSTParser<'a, O>, kind: SyntaxKind) -> impl CSTParser<'a, ()> {
    ws().then(rowan_leaf(kind, parser)).map(|_| ())
}

struct RowanNode_<'a, O, P: CSTParser<'a, O>> {
    parser: P,
    kind: SyntaxKind,
    _marker: PhantomData<(&'a str, fn() -> O)>,
}

type RowanNode<'a, O, P> = Ext<RowanNode_<'a, O, P>>;

/// This needs to be an extension, not a combinator using `map_with`, because map_with can be evaluated multiple times in the case of backtracking.
impl<'a, O, P: CSTParser<'a, O>> ExtParser<'a, &'a str, (), CSTExtra<'a>> for RowanNode_<'a, O, P> {
    fn parse(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let checkpoint = inp.state().checkpoint();

        inp.parse(&self.parser)?;
        let builder = inp.state();
        builder.start_node_at(checkpoint, self.kind.into());
        builder.finish_node();
        Ok(())
    }

    fn check(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let checkpoint = inp.state().checkpoint();

        inp.check(&self.parser)?;
        let builder = inp.state();
        builder.start_node_at(checkpoint, self.kind.into());
        builder.finish_node();
        Ok(())
    }
}

fn node<'a, O, P: CSTParser<'a, O>>(kind: SyntaxKind, parser: P) -> RowanNode<'a, O, P> {
    Ext(RowanNode_ {
        parser,
        kind,
        _marker: PhantomData,
    })
}

struct RowanLeaf_<'a, O, P: CSTParser<'a, O>> {
    parser: P,
    kind: SyntaxKind,
    _marker: PhantomData<(&'a str, fn() -> O)>,
}

type RowanLeaf<'a, O, P> = Ext<RowanLeaf_<'a, O, P>>;

/// This needs to be an extension, not a combinator using `map_with`, because map_with isn't evaluated when chumsky notices the output isn't used.
impl<'a, O, P: CSTParser<'a, O>> ExtParser<'a, &'a str, (), CSTExtra<'a>> for RowanLeaf_<'a, O, P> {
    fn parse(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let start = inp.offset();
        inp.parse(&self.parser)?;
        let text = inp.slice_since(start..);
        inp.state().token(self.kind.into(), text);
        Ok(())
    }

    fn check(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let start = inp.offset();
        inp.check(&self.parser)?;
        let text = inp.slice_since(start..);
        inp.state().token(self.kind.into(), text);
        Ok(())
    }
}

fn rowan_leaf<'a, O, P: CSTParser<'a, O>>(kind: SyntaxKind, parser: P) -> RowanLeaf<'a, O, P> {
    Ext(RowanLeaf_ {
        parser,
        kind,
        _marker: PhantomData,
    })
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
    fn colon: COLON = just(':');
}

#[rustfmt::skip]
fn parser<'a>() -> impl CSTParser<'a> {
    choice((
        ws().then(nl()).map(|_| ()),
        statement(),
    ))
    .repeated().count().map(|_| ()).then_ignore(end())
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

fn cell_range<'a>() -> impl CSTParser<'a> {
    node(CELL_RANGE, cell().then(colon()).then(cell()))
}

fn place<'a>() -> impl CSTParser<'a> {
    node(PLACE, cell())
    // node(PLACE, choice((cell_range(), cell())))
}

// A1 = 3
fn assign<'a>() -> impl CSTParser<'a> {
    node(ASSIGN, place().then(eq()).then(int()))
}

fn statement<'a>() -> impl CSTParser<'a> {
    node(STATEMENT, assign().then(nl()))
}
