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
use SyntaxKind::*;

type CSTError<'a> = Simple<'a, char>;
type CSTExtra<'a> = extra::Full<CSTError<'a>, (), ()>;
// trait CSTParser<'a, O = ()> = chumsky::Parser<'a, &'a str, O, extra::Default>;
trait CSTParser<'a, O = ()> =
    chumsky::Parser<'a, &'a str, O, extra::Full<CSTError<'a>, GreenNodeBuilder<'static>, ()>>;

#[derive(Debug)]
pub struct Parse {
    root: GreenNode,
    errors: Vec<CSTError<'static>>,
}

impl Parse {
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

pub fn parse(text: String) -> Parse {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(ROOT.into());
    let errors = parser()
        // TODO: lol. lmao.
        .parse_with_state(text.leak(), &mut builder)
        .into_errors();
    builder.finish_node();
    Parse {
        root: builder.finish(),
        errors,
    }
}

// struct ParseBuilder {
//     builder: GreenNodeBuilder<'static>,
// }

// trait AsStr {
//     fn as_str(&self) -> &str;
// }

// impl AsStr for u8 {
//     fn as_str(&self) -> &str {
//         std::str::from_utf8(std::slice::from_ref(self)).unwrap()
//     }
// }

// impl AsStr for str {
//     fn as_str(&self) -> &str {
//         self
//     }
// }

// impl ParseBuilder {

// struct Leaf_<P> {
//     parser: P,
//     kind: SyntaxKind,
// }

// impl<'a, P: CSTParser<'a>> ExtParser<'a, &'a str, (), CSTExtra<'a>> for Leaf_<P> {
//     fn parse(
//         &self,
//         _: &mut chumsky::input::InputRef<'a, '_, &'a str, CSTExtra<'a>>,
//     ) -> Result<(), CSTError<'a>> {
//         // self.parser.map_with(move |s, extra| {
//             let builder = extra.state();
//             builder.token(self.kind.into(), s);
//         // })
//     }
// }

// type Leaf<P> = Ext<Leaf_<P>>;

// // fn leaf(k

// fn leaf<'a>(parser: impl CSTParser<'a, &'a str>, kind: SyntaxKind) -> impl CSTParser<'static, ()> {
//     Ext(Leaf_ { parser, kind })
// }
fn leaf<'a, O>(parser: impl CSTParser<'a, O>, kind: SyntaxKind) -> impl CSTParser<'static, ()> {
    parser.map_with(move |_, extra| {
        let slice = extra.slice();
        extra.state().token(kind.into(), slice);
    })
}
// // }

#[rustfmt::skip]
fn parser() -> impl CSTParser<'static> {
    choice((
        leaf(just('\n'), NEWLINE),
        leaf(just('='), EQ),
        leaf(text::digits(10), INT),
    )).then_ignore(end())
}

// AAA123
fn cell<'a>() -> impl CSTParser<'a, &'a str> {
    any()
        .filter(char::is_ascii_alphabetic)
        .repeated()
        .at_least(1)
        .then(any().filter(char::is_ascii_digit).repeated().at_least(1))
        .to_slice()
}
