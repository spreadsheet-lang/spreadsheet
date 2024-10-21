use std::{collections::HashSet, path::Path};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse2;
use ungrammar::{Grammar, Node, Rule, Token};

fn main() {
    let grammar: Grammar = include_str!("ast.ungram").parse().unwrap();
    let mut walker = GrammarWalker {
        ast: AstSrc::default(),
    };
    for n in grammar.iter() {
        walker.lower_node(&grammar, n);
    }
    println!("{:?}", walker.ast.nodes);
    let src = generate(walker.ast);
    println!("{src}");
    let pretty = prettyplease::unparse(&parse2(src).unwrap());
    println!("{pretty}");
    let dst = Path::new(&std::env::var("OUT_DIR").unwrap()).join("ast.rs");
    std::fs::write(dst, pretty).unwrap();
}

/// It's hard to generate tokens as-we-go. Instead add an IR for items in the ungrammar.
#[derive(Default, Debug)]
struct AstSrc {
    tokens: HashSet<String>,
    nodes: Vec<AstItem>,
}

#[derive(Debug)]
struct AstItem {
    name: String,
    kind: AstItemKind,
}

#[derive(Debug)]
enum AstItemKind {
    Struct(Vec<Field>),
    Enum(Vec<Variant>),
}

#[derive(Debug)]
enum Variant {
    Token(String),
    Node(String),
}

#[derive(Debug)]
enum Field {
    Token,
    Node {
        label: String,
        ty: String,
        cardinality: Cardinality,
    },
}

#[derive(Debug)]
enum Cardinality {
    /// When we recover from errors we may not have some of the fields
    Optional,
    Many,
}

use Cardinality::*;

struct GrammarWalker {
    ast: AstSrc,
}

impl GrammarWalker {
    fn lower_node(&mut self, grammar: &Grammar, n: Node) {
        let mut acc = vec![];
        let node = &grammar[n];
        self.lower_rule(grammar, &mut acc, &node.rule, &node.name, None);
        if acc.is_empty() {
            return; // enum or data-less token
        }
        self.ast.nodes.push(AstItem {
            name: node.name.clone(),
            kind: AstItemKind::Struct(acc),
        });
    }

    fn lower_token(&mut self, grammar: &Grammar, t: Token) -> Option<Field> {
        let token = &grammar[t];
        let Some(kind) = token.name.strip_prefix('#') else {
            return None;
        };
        self.ast.tokens.insert(kind.to_owned());
        Some(Field::Token)
    }

    fn lower_rule(
        &mut self,
        grammar: &Grammar,
        acc: &mut Vec<Field>,
        rule: &Rule,
        name: &str,
        label: Option<&str>,
    ) {
        let field = match *rule {
            Rule::Labeled {
                ref label,
                ref rule,
            } => {
                self.lower_rule(grammar, acc, rule, name, Some(label));
                return;
            }
            Rule::Node(n) => {
                let nested = &grammar[n];
                let name = match label {
                    Some(s) => s.into(),
                    None => to_lower_snake_case(&nested.name),
                };
                Field::Node {
                    label: name,
                    ty: nested.name.clone(),
                    cardinality: Optional,
                }
            }
            Rule::Token(t) => match self.lower_token(grammar, t) {
                Some(field) => field,
                None => return,
            },
            Rule::Rep(ref inner) => match **inner {
                Rule::Node(n) => {
                    let ty = &grammar[n].name;
                    let name = match label {
                        Some(s) => s.into(),
                        None => pluralize(&to_lower_snake_case(&ty)),
                    };
                    Field::Node {
                        label: name,
                        ty: ty.into(),
                        cardinality: Many,
                    }
                }
                Rule::Token(t) => {
                    let Some(field) = self.lower_token(grammar, t) else {
                        return;
                    };
                    field
                }
                _ => panic!(
                    "{}: unhandled Rule::Rep (repeated with '*'): {:?}\nInner: {:?}",
                    name, rule, inner
                ),
            },
            Rule::Alt(ref rules) => {
                let mut variants = vec![];
                for rule in rules {
                    match *rule {
                        Rule::Node(n) => {
                            let ty = &grammar[n].name;
                            variants.push(Variant::Node(ty.clone()));
                        }
                        Rule::Token(n) => {
                            let ty = &grammar[n].name;
                            if let Some(data) = ty.strip_prefix('#') {
                                self.ast.tokens.insert(data.to_owned());
                                variants.push(Variant::Token(to_pascal_case(data)));
                            }
                        }
                        _ => panic!("unhandled variant {rule:?} for enum {name}"),
                    }
                }
                self.ast.nodes.push(AstItem {
                    name: name.into(),
                    kind: AstItemKind::Enum(variants),
                });
                return;
            }
            Rule::Opt(ref rule) => return self.lower_rule(grammar, acc, rule, name, label),
            Rule::Seq(ref rules) => {
                for rule in rules {
                    self.lower_rule(grammar, acc, rule, name, label);
                }
                return;
            }
        };
        acc.push(field);
    }
}

fn generate(ast: AstSrc) -> TokenStream {
    let mut acc = quote! {
        use crate::parser::{SyntaxKind, SyntaxNode};
        use ::cstree::green::GreenNode;
    };
    for node in &ast.nodes {
        let name = format_ident!("{}", node.name);
        let kind = format_ident!("{}", to_upper_snake_case(&node.name));
        let item = match &node.kind {
            AstItemKind::Struct(fields) => {
                let fields = fields.iter().map(|f| match f {
                    Field::Token { .. } => quote! {},
                    Field::Node {
                        label: name,
                        ty,
                        cardinality,
                    } => {
                        let name = format_ident!("{name}");
                        let ty = format_ident!("{ty}");
                        let (ret_ty, mapper) = match cardinality {
                            Many => (quote! { Vec<#ty> }, quote! { .collect() }),
                            Optional => (quote! { Option<#ty> }, quote! { .next() }),
                        };
                        quote! {
                           fn #name(&self) -> #ret_ty {
                               self.syntax.children().filter_map(|n| #ty::cast(n.clone())) #mapper
                          }
                        }
                    }
                });
                quote! {
                   #[derive(Debug)]
                   struct #name {
                       syntax: SyntaxNode,
                   }
                   impl #name {
                       fn can_cast(kind: SyntaxKind) -> bool {
                           kind == SyntaxKind::#kind
                       }
                       fn cast(syntax: SyntaxNode) -> Option<Self> {
                           if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }
                       }
                       #(#fields)*
                   }
                }
            }
            AstItemKind::Enum(variants) => {
                let (variants, names): (Vec<_>, Vec<_>) = variants
                    .iter()
                    .map(|v| match v {
                        Variant::Node(name) => {
                            let kind = format_ident!("{}", to_upper_snake_case(name));
                            let name = format_ident!("{name}");
                            let variant = quote! {
                                #name(#name)
                            };
                            (variant, (name, kind))
                        }
                        Variant::Token(name) => {
                            let kind = format_ident!("{}", to_upper_snake_case(name));
                            let ty = format_ident!("{}", to_pascal_case(name));
                            let name = format_ident!("{name}");
                            let variant = quote! {
                                #name(#ty)
                            };
                            (variant, (name, kind))
                        }
                    })
                    .unzip();
                let (nested_enums, structs): (Vec<_>, Vec<_>) =
                    names.into_iter().partition(|(name, _)| {
                        ast.nodes
                            .iter()
                            // TODO: wrong for raw idents
                            .any(|item| {
                                matches!(item.kind, AstItemKind::Enum(..))
                                    && item.name == name.to_string()
                            })
                    });
                let (names, kinds): (Vec<_>, Vec<_>) = structs.into_iter().unzip();
                let (enums, _): (Vec<_>, Vec<_>) = nested_enums.into_iter().unzip();
                quote! {
                   #[derive(Debug)]
                    enum #name {
                        #(#variants),*
                    }
                    impl #name {
                        fn can_cast(kind: SyntaxKind) -> bool {
                            matches!(kind, #(SyntaxKind::#kinds)|*)
                        }
                        fn cast(syntax: SyntaxNode) -> Option<Self> {
                            let res = match syntax.kind() {
                                #(
                                    SyntaxKind::#kinds => Self::#names(#names { syntax }),
                                )*
                                _ => return None #(.or_else(|| #enums::cast(syntax.clone()).map(Self::#enums)))*
                            };
                            Some(res)
                        }
                    }
                }
            }
        };
        acc.extend(item);
    }

    let tokens = ast
        .tokens
        .iter()
        .map(|t| format_ident!("{}", to_pascal_case(t)));
    acc.extend(quote! {
        #(
            #[derive(Debug)]
            struct #tokens { syntax: SyntaxNode }
        )*
    });
    acc
}

// blatently taken from rust-analyzer
fn pluralize(s: &str) -> String {
    format!("{s}s")
}

fn to_lower_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_lowercase());
    }
    buf
}

fn to_upper_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_uppercase());
    }
    buf
}

fn to_pascal_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev_is_underscore = true;
    for c in s.chars() {
        if c == '_' {
            prev_is_underscore = true;
        } else if prev_is_underscore {
            buf.push(c.to_ascii_uppercase());
            prev_is_underscore = false;
        } else {
            buf.push(c.to_ascii_lowercase());
        }
    }
    buf
}
