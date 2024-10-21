use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse2;
use ungrammar::{Grammar, Node, Rule};

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
///
/// I really wish this were part of `ungrammar` itself...
#[derive(Default, Debug)]
struct AstSrc {
    // tokens: Vec<String>,
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
    Token(String),
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
            return; // this was an enum
        }
        self.ast.nodes.push(AstItem {
            name: node.name.clone(),
            kind: AstItemKind::Struct(acc),
        });
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
            Rule::Token(t) => {
                let token = &grammar[t];
                let Some(kind) = token.name.strip_prefix('#') else {
                    return;
                };
                let name = match label {
                    Some(s) => s.into(),
                    None => to_lower_snake_case(kind),
                };
                Field::Token(name)
            }
            Rule::Rep(ref inner) => {
                if let Rule::Node(n) = **inner {
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
                } else {
                    panic!(
                        "{}: unhandled Rule::Rep (repeated with '*'): {:?}\nInner: {:?}",
                        name, rule, inner
                    )
                }
            }
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
    let mut acc = quote! { use ::cstree::green::GreenNode; };
    for node in ast.nodes {
        let name = format_ident!("{}", node.name);
        let item = match node.kind {
            AstItemKind::Struct(fields) => {
                let fields = fields.iter().map(|f| match f {
                    Field::Token(name) => {
                        let name = format_ident!("{name}");
                        quote! {
                            #name: GreenNode,
                        }
                    }
                    Field::Node {
                        label: name,
                        ty,
                        cardinality,
                    } => {
                        let name = format_ident!("{name}");
                        let ty = format_ident!("{ty}");
                        let ty = match cardinality {
                            Many => quote! { Vec<#ty> },
                            Optional => quote! { Option<#ty> },
                        };
                        quote! {
                           #name: #ty,
                        }
                    }
                });
                quote! {
                   struct #name {
                       #(#fields)*
                   }
                }
            }
            AstItemKind::Enum(variants) => {
                let variants = variants.iter().map(|v| match v {
                    Variant::Node(name) => {
                        let name = format_ident!("{name}");
                        quote! {
                            #name(#name)
                        }
                    }
                    Variant::Token(name) => {
                        let name = format_ident!("{name}");
                        quote! {
                            #name(GreenNode)
                        }
                    }
                });
                quote! {
                    enum #name {
                        #(#variants),*
                    }
                }
            }
        };
        acc.extend(item);
    }
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
