//! We use a concrete syntax tree (CST) represented by a rowan red-green tree
//! and parsed with chumsky.
//!
//! Unlike most ASTs, CSTs preserve comments and whitespace. Additionally,
//! Rowan particularly does not distinguish trivia from regular nodes, and uses
//! the same enum for both leaf nodes (tokens) and composite nodes (trees).
//!
//! This module is "glue code" between rowan and chumsky.
//! The actual parser lives in `grammar.rs`.

// separate mod to encapsulate the unsafety
mod syntax {
    use cstree::RawSyntaxKind;

    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[allow(non_camel_case_types)]
    pub enum SyntaxKind {
        // leaf nodes
        // symbols
        NEWLINE = 0,
        CELL,
        EQ,
        INT,
        COLON,
        DOLLAR,
        // LEFT_BRACKET,
        // RIGHT_BRACKET,

        // keywords
        ALIAS_TOK,
        ENUM_TOK,

        // data tokens
        WHITESPACE,
        IDENT,
        STR,

        // composite nodes

        // expressions
        CELL_RANGE,
        PLACE,
        ALIAS_EXPR,
        ENUM_EXPR,
        EXPR,

        // statements
        ASSIGN,
        ALIAS_STMT,
        STATEMENT,
        // ARRAY_RANGE,

        // this MUST come last in the enum; we depend on it for memory safety
        ROOT,
    }

    impl From<SyntaxKind> for RawSyntaxKind {
        fn from(kind: SyntaxKind) -> Self {
            Self(kind as u32)
        }
    }

    impl cstree::Syntax for SyntaxKind {
        fn from_raw(raw: RawSyntaxKind) -> Self {
            assert!(raw.0 <= Self::ROOT as u32);
            // SAFETY: we just checked this is a valid variant.
            unsafe { std::mem::transmute::<u32, SyntaxKind>(raw.0) }
        }
        fn into_raw(self) -> RawSyntaxKind {
            self.into()
        }

        fn static_text(self) -> Option<&'static str> {
            None
        }
    }

    pub type SyntaxNode = cstree::syntax::SyntaxNode<SyntaxKind>;
    pub type SyntaxToken = cstree::syntax::SyntaxToken<SyntaxKind>;
    pub type SyntaxElement = cstree::util::NodeOrToken<SyntaxNode, SyntaxToken>;
}

use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use cstree::interning::TokenInterner;
pub use syntax::*;

use chumsky::{
    extension::v1::{Ext, ExtParser},
    input::InputRef,
    prelude::*,
};
use cstree::build::GreenNodeBuilder;
use cstree::green::GreenNode;

pub(crate) type CSTError<'a> = Simple<'a, char>;
pub(crate) type CSTExtra<'a> = extra::Full<CSTError<'a>, RowanRecorder<'a>, ()>;
pub(crate) trait CSTParser<'a, O = ()>:
    chumsky::Parser<'a, &'a str, O, CSTExtra<'a>>
{
}
impl<'a, O, T> CSTParser<'a, O> for T where T: chumsky::Parser<'a, &'a str, O, CSTExtra<'a>> {}

pub(crate) struct RowanRecorder<'a> {
    builder: GreenNodeBuilder<'a, 'static, SyntaxKind>,
}

impl<'a> Default for RowanRecorder<'a> {
    fn default() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
        }
    }
}

impl<'a> chumsky::recorder::Recorder<'a, &'a str> for RowanRecorder<'a> {
    type SaveMarker = cstree::build::Checkpoint;

    fn on_token(&mut self, _: char) {}

    fn on_save<'parse>(&self, _: <&'parse str as Input<'parse>>::Offset) -> Self::SaveMarker {
        self.builder.checkpoint()
    }

    fn on_rewind<'parse>(
        &mut self,
        marker: chumsky::input::Marker<'a, 'parse, &'a str, Self::SaveMarker>,
    ) {
        self.builder.revert(marker.ext_checkpoint())
    }
}

impl<'a> Deref for RowanRecorder<'a> {
    type Target = GreenNodeBuilder<'a, 'static, SyntaxKind>;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

impl<'a> DerefMut for RowanRecorder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.builder
    }
}

pub struct Parse<'a> {
    pub root: GreenNode,
    interner: TokenInterner,
    pub errors: Vec<CSTError<'a>>,
}

impl<'a> fmt::Debug for Parse<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.errors.is_empty() {
            writeln!(f, "error: {:?}", self.errors)?;
        }
        self.red_tree().write_debug(&self.interner, f, true)
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
}

pub fn parse(text: &str) -> Parse {
    let mut builder = RowanRecorder {
        builder: GreenNodeBuilder::new(),
    };
    // we don't put this in parser() to ensure rowan never panics even on horribly invalid programs
    builder.start_node(SyntaxKind::ROOT.into());
    let errors = crate::grammar::parser()
        .parse_with_state(text, &mut builder)
        .into_errors();
    builder.finish_node();
    let (root, interner) = builder.builder.finish();
    // dbg!(&errors, &builder);
    Parse {
        root,
        interner: interner.unwrap().into_interner().unwrap(),
        errors,
    }
}

pub(crate) struct RowanNode_<'a, O, P: CSTParser<'a, O>> {
    parser: P,
    kind: SyntaxKind,
    debug: bool,
    _marker: PhantomData<(&'a str, fn() -> O)>,
}

pub(crate) type RowanNode<'a, O, P> = Ext<RowanNode_<'a, O, P>>;

/// This needs to be an extension, not a combinator using `map_with`, because map_with can be evaluated multiple times in the case of backtracking.
impl<'a, O, P: CSTParser<'a, O>> ExtParser<'a, &'a str, (), CSTExtra<'a>> for RowanNode_<'a, O, P> {
    fn parse(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let checkpoint = inp.state().checkpoint();
        if self.debug {
            println!("node start {:?} {checkpoint:?}", self.kind);
        }

        inp.parse(&self.parser)?;
        let builder = inp.state();
        builder.start_node_at(checkpoint, self.kind.into());
        builder.finish_node();
        if self.debug {
            println!("node finish {:?} {checkpoint:?}", self.kind);
        }
        Ok(())
    }

    fn check(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let checkpoint = inp.state().checkpoint();
        if self.debug {
            println!("(check) node start {:?} {checkpoint:?}", self.kind);
        }

        inp.check(&self.parser)?;
        let builder = inp.state();
        builder.start_node_at(checkpoint, self.kind.into());
        builder.finish_node();
        if self.debug {
            println!("(check) node finish {:?} {checkpoint:?}", self.kind);
        }
        Ok(())
    }
}

pub(crate) fn rowan_node<'a, O, P: CSTParser<'a, O>>(
    kind: SyntaxKind,
    parser: P,
) -> RowanNode<'a, O, P> {
    Ext(RowanNode_ {
        parser,
        kind,
        debug: option_env!("SSL_DEBUG").is_some(),
        _marker: PhantomData,
    })
}

pub(crate) struct RowanLeaf_<'a, O, P: CSTParser<'a, O>> {
    parser: P,
    kind: SyntaxKind,
    debug: bool,
    _marker: PhantomData<(&'a str, fn() -> O)>,
}

pub(crate) type RowanLeaf<'a, O, P> = Ext<RowanLeaf_<'a, O, P>>;

/// This needs to be an extension, not a combinator using `map_with`, because map_with isn't evaluated when chumsky notices the output isn't used.
impl<'a, O, P: CSTParser<'a, O>> ExtParser<'a, &'a str, (), CSTExtra<'a>> for RowanLeaf_<'a, O, P> {
    fn parse(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let start = inp.offset();
        inp.parse(&self.parser)?;
        let text = inp.slice_since(start..);
        // need this to handle `or_not`
        if !text.is_empty() {
            if self.debug {
                println!("token {:?}", self.kind);
            }
            inp.state().token(self.kind.into(), text);
        }
        Ok(())
    }

    fn check(&self, inp: &mut InputRef<'a, '_, &'a str, CSTExtra<'a>>) -> Result<(), CSTError<'a>> {
        let start = inp.offset();
        inp.check(&self.parser)?;
        let text = inp.slice_since(start..);
        if !text.is_empty() {
            if self.debug {
                println!("(check) token {:?}", self.kind);
            }
            inp.state().token(self.kind.into(), text);
        }
        Ok(())
    }
}

pub(crate) fn rowan_leaf<'a, O, P: CSTParser<'a, O>>(
    kind: SyntaxKind,
    parser: P,
) -> RowanLeaf<'a, O, P> {
    Ext(RowanLeaf_ {
        parser,
        kind,
        debug: option_env!("SSL_DEBUG").is_some(),
        _marker: PhantomData,
    })
}
