//! We use a concrete syntax tree (CST) represented by a rowan red-green tree
//! and parsed with chumsky.
//!
//! Unlike most ASTs, CSTs preserve comments and whitespace. Additionally,
//! Rowan particularly does not distinguish trivia from regular nodes, and uses
//! the same enum for both leaf nodes (tokens) and composite nodes (trees).
//!
//! This module is "glue code" between rowan and chumsky.
//! The actual parser lives in `grammar.rs`.

use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Range},
};

use chumsky::{
    extension::v1::{Ext, ExtParser},
    input::{BoxedStream, InputRef, SpannedInput, Stream, ValueInput},
    prelude::*,
};
use cstree::{build::GreenNodeBuilder, green::GreenNode, interning::TokenInterner};
use logos::Logos;

use crate::grammar::{self, Token};

// separate mod to encapsulate the unsafety
mod syntax {
    use cstree::RawSyntaxKind;

    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[allow(non_camel_case_types)]
    pub enum SyntaxKind {
        // leaf nodes
        NEWLINE = 0,

        // symbols
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
        COMMENT,
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
        COMMENTED_STATEMENT,
        // ARRAY_RANGE,
        SOURCE_FILE,

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
    #[allow(dead_code)]
    pub type SyntaxToken = cstree::syntax::SyntaxToken<SyntaxKind>;
    #[allow(dead_code)]
    pub type SyntaxElement = cstree::util::NodeOrToken<SyntaxNode, SyntaxToken>;
}

pub use syntax::*;

pub(crate) type CSTError<'a> = Rich<'a, Token>;
pub(crate) type CSTExtra<'a> = extra::Full<CSTError<'a>, RowanRecorder<'a>, ()>;
pub(crate) trait CSTParser<
    'a,
    O = (),
    I: ValueInput<'a, Token = Token, Span = SimpleSpan> = SpannedInput<
        Token,
        SimpleSpan,
        BoxedStream<'a, (Token, SimpleSpan)>,
    >,
>: chumsky::Parser<'a, I, O, CSTExtra<'a>>
{
}

impl<'a, I, O, T> CSTParser<'a, O, I> for T
where
    T: chumsky::Parser<'a, I, O, CSTExtra<'a>>,
    I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
}

pub(crate) struct RowanRecorder<'a> {
    src: &'a str,
    builder: GreenNodeBuilder<'a, 'static, SyntaxKind>,
}

type CSTInput<'a> = SpannedInput<Token, SimpleSpan, BoxedStream<'a, (Token, SimpleSpan)>>;

impl<'a, I> chumsky::inspector::Inspector<'a, I> for RowanRecorder<'a>
where
    I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
    type SaveMarker = cstree::build::Checkpoint;

    fn on_token(&mut self, _: &Token) {}

    fn on_save<'parse>(&self, _: I::Offset) -> Self::SaveMarker {
        let checkpoint = self.builder.checkpoint();
        if option_env!("SSL_DEBUG").is_some() {
            println!("save {checkpoint:?}");
        }
        checkpoint
    }

    fn on_rewind<'parse>(
        &mut self,
        marker: chumsky::input::Marker<'a, 'parse, I, Self::SaveMarker>,
    ) {
        if option_env!("SSL_DEBUG").is_some() {
            println!("rollback {:?}", marker.ext_checkpoint());
        }
        self.builder.revert_to(marker.ext_checkpoint())
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
    pub interner: TokenInterner,
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
    parse_node(text, grammar::file())
}

pub(crate) fn parse_node<'a, P>(text: &'a str, parser: P) -> Parse<'a>
where
    P: CSTParser<'a, ()>,
{
    let tokens = Token::lexer(text).spanned().map(|(tok, span)| match tok {
        Ok(tok) => {
            if option_env!("SSL_DEBUG").is_some() {
                eprintln!("lexer: tok={tok:?}@{span:?} {:?}", &text[span.clone()]);
            }
            (tok, span.into())
        }
        Err(()) => (Token::Error, span.into()),
    });
    let stream = Stream::from_iter(tokens)
        .boxed()
        .spanned(SimpleSpan::splat(text.len()));
    let mut recorder = RowanRecorder {
        src: &text,
        builder: GreenNodeBuilder::new(),
    };

    // NOTE: cstree requires that our root node has exactly one child.
    // Additionally, in some unit tests we pass in individual tokens, which isn't allowed.
    // Rather than try to be smart, just wrap every possible parse tree in a ROOT node.
    recorder.start_node(SyntaxKind::ROOT.into());
    let errors = parser.parse_with_state(stream, &mut recorder).into_errors();
    recorder.finish_node();

    let (root, interner) = recorder.builder.finish();

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
    _marker: PhantomData<(CSTInput<'a>, fn() -> O)>,
}

pub(crate) type RowanNode<'a, O, P> = Ext<RowanNode_<'a, O, P>>;

/// This needs to be an extension, not a combinator using `map_with`, because map_with can be evaluated multiple times in the case of backtracking.
impl<'a, O, P: CSTParser<'a, O>> ExtParser<'a, CSTInput<'a>, (), CSTExtra<'a>>
    for RowanNode_<'a, O, P>
{
    // WARNING: keep this in sync with check()
    fn parse(
        &self,
        inp: &mut InputRef<'a, '_, CSTInput<'a>, CSTExtra<'a>>,
    ) -> Result<(), CSTError<'a>> {
        let checkpoint = inp.state().checkpoint();
        if self.debug {
            println!("node start {:?} {checkpoint:?}", self.kind);
        }

        inp.parse(&self.parser)?;
        let builder = &mut inp.state().builder;
        builder.start_node_at(checkpoint, self.kind.into());
        builder.finish_node();
        if self.debug {
            println!("node finish {:?} {checkpoint:?}", self.kind);
        }
        Ok(())
    }

    // WARNING: keep this in sync with parse()
    fn check(
        &self,
        inp: &mut InputRef<'a, '_, CSTInput<'a>, CSTExtra<'a>>,
    ) -> Result<(), CSTError<'a>> {
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
    _marker: PhantomData<(CSTInput<'a>, fn() -> O)>,
}

pub(crate) type RowanLeaf<'a, O, P> = Ext<RowanLeaf_<'a, O, P>>;

/// This needs to be an extension, not a combinator using `map_with`, because map_with isn't evaluated when chumsky notices the output isn't used.
impl<'a, O, P: CSTParser<'a, O>> ExtParser<'a, CSTInput<'a>, (), CSTExtra<'a>>
    for RowanLeaf_<'a, O, P>
{
    // WARNING: keep this in sync with check()
    fn parse(
        &self,
        inp: &mut InputRef<'a, '_, CSTInput<'a>, CSTExtra<'a>>,
    ) -> Result<(), CSTError<'a>> {
        let start = inp.offset();
        inp.parse(&self.parser)?;

        // HACK: chumsky is buggy and always gives us back at least one token,
        // even when we used or_not to avoid eating a token. override what it
        // thinks a span is.
        if start == inp.offset() {
            return Ok(());
        }

        let span: Range<usize> = inp.span_since(start).into();
        let text: &str = &inp.state().src[span.clone()];
        if self.debug {
            println!("token {:?} {:} (offset={:?})", self.kind, text, span,);
        }
        inp.state().token(self.kind.into(), text);
        Ok(())
    }

    // WARNING: keep this in sync with parse()
    fn check(
        &self,
        inp: &mut InputRef<'a, '_, CSTInput<'a>, CSTExtra<'a>>,
    ) -> Result<(), CSTError<'a>> {
        let start = inp.offset();
        inp.check(&self.parser)?;

        // HACK: chumsky is buggy and always gives us back at least one token,
        // even when we used or_not to avoid eating a token. override what it
        // thinks a span is.
        if start == inp.offset() {
            return Ok(());
        }

        let span: Range<usize> = inp.span_since(start).into();
        let text: &str = &inp.state().src[span.clone()];
        if self.debug {
            println!("token {:?} {:} (offset={:?})", self.kind, text, span,);
        }
        inp.state().token(self.kind.into(), text);
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
