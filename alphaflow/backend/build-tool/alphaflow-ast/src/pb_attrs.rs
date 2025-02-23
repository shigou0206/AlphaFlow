#![allow(clippy::all)]

use crate::symbol::*;
use crate::ctxt::ASTResult;
use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    self,
    parse::{self, Parse},
    Meta::{List, NameValue, Path},
    NestedMeta::{Lit, Meta},
};

/// ----------------------------------------------------------------------------
/// PBAttrsContainer
/// ----------------------------------------------------------------------------

/// Extracted information from the `#[pb(...)]` attribute on a type (struct / enum),
/// mainly used to specify the type name (pb_struct_type / pb_enum_type) when generating Protobuf.
///
/// - `name`: Original type name (identifier in Rust)
/// - `pb_struct_type`: If `#[pb(struct="Type")]` is written on a struct, the parsed syn::Type
/// - `pb_enum_type`: If `#[pb(enum="Type")]` is written on an enum, the parsed syn::Type
///
/// If the user does not explicitly write `#[pb(struct="X")]` or `#[pb(enum="X")]`, the Rust ident is used as the default proto type name.
pub struct PBAttrsContainer {
    _name: String,
    pb_struct_type: Option<syn::Type>,
    pb_enum_type: Option<syn::Type>,
}

impl PBAttrsContainer {
    /// Parse the `#[pb(...)]` attribute from a `syn::DeriveInput` (representing a struct/enum/union).
    ///
    /// - If item.data is a struct, pb_struct_type defaults to ident
    /// - If item.data is an enum, pb_enum_type defaults to ident
    /// - If `#[pb(struct="SomeType")]` or `#[pb(enum="SomeType")]` is explicitly written, it overrides the default value
    pub fn from_ast(ast_result: &ASTResult, item: &syn::DeriveInput) -> Self {
        let mut pb_struct_type = ASTAttr::none(ast_result, PB_STRUCT);
        let mut pb_enum_type   = ASTAttr::none(ast_result, PB_ENUM);

        // Collect #[pb(...)] meta items
        for meta_item in item
            .attrs
            .iter()
            .flat_map(|attr| get_pb_meta_items(ast_result, attr))
            .flatten()
        {
            match &meta_item {
                // #[pb(struct="...")]
                Meta(NameValue(m)) if m.path == PB_STRUCT => {
                    if let Ok(ty) = parse_lit_into_ty(ast_result, PB_STRUCT, &m.lit) {
                        pb_struct_type.set_opt(&m.path, Some(ty));
                    }
                },
                // #[pb(enum="...")]
                Meta(NameValue(m)) if m.path == PB_ENUM => {
                    if let Ok(ty) = parse_lit_into_ty(ast_result, PB_ENUM, &m.lit) {
                        pb_enum_type.set_opt(&m.path, Some(ty));
                    }
                },
                // Unknown key => error
                Meta(meta_item) => {
                    let path = meta_item.path().into_token_stream().to_string().replace(' ', "");
                    ast_result.error_spanned_by(
                        meta_item.path(),
                        format!("unknown container attribute `{}`", path),
                    );
                },
                // Pure literal => error
                Lit(lit) => {
                    ast_result.error_spanned_by(lit, "unexpected literal in container attribute");
                },
            }
        }

        // If struct, pb_struct_type not specified => default to ident
        // If enum, pb_enum_type not specified => default to ident
        match &item.data {
            syn::Data::Struct(_) => {
                pb_struct_type.set_if_none(default_pb_type(ast_result, &item.ident));
            },
            syn::Data::Enum(_) => {
                pb_enum_type.set_if_none(default_pb_type(ast_result, &item.ident));
            },
            _ => {
                // union is not processed
            },
        }

        PBAttrsContainer {
            _name: item.ident.to_string(),
            pb_struct_type: pb_struct_type.get(),
            pb_enum_type: pb_enum_type.get(),
        }
    }

    /// If it is a struct, it may have Some(syn::Type), otherwise None
    pub fn pb_struct_type(&self) -> Option<&syn::Type> {
        self.pb_struct_type.as_ref()
    }

    /// If it is an enum, it may have Some(syn::Type), otherwise None
    pub fn pb_enum_type(&self) -> Option<&syn::Type> {
        self.pb_enum_type.as_ref()
    }

    /// Returns the name of the type
    pub fn name(&self) -> &str {
        &self._name
    }
}

/// ----------------------------------------------------------------------------
/// ASTAttr
/// ----------------------------------------------------------------------------

/// General container: stores an attribute value (such as "int literal" or "expr path") during parsing
/// and can report an error if duplicate annotations are found.
pub struct ASTAttr<'c, T> {
    ast_result: &'c ASTResult,
    name: Symbol,
    tokens: TokenStream,
    value: Option<T>,
}

impl<'c, T> ASTAttr<'c, T> {
    /// Create an empty ASTAttr with no initial value
    pub(crate) fn none(ast_result: &'c ASTResult, name: Symbol) -> Self {
        ASTAttr {
            ast_result,
            name,
            tokens: TokenStream::new(),
            value: None,
        }
    }

    /// Set the attribute value, report a duplicate error if a value already exists
    pub(crate) fn set<A: ToTokens>(&mut self, obj: A, value: T) {
        let tokens = obj.into_token_stream();
        if self.value.is_some() {
            self.ast_result.error_spanned_by(
                tokens,
                format!("duplicate attribute `{}`", self.name),
            );
        } else {
            self.tokens = tokens;
            self.value = Some(value);
        }
    }

    /// Use set_opt when the value needs to be set only in certain cases
    fn set_opt<A: ToTokens>(&mut self, obj: A, value: Option<T>) {
        if let Some(val) = value {
            self.set(obj, val);
        }
    }

    /// Set the value if there is currently no value
    pub(crate) fn set_if_none(&mut self, value: T) {
        if self.value.is_none() {
            self.value = Some(value);
        }
    }

    /// Consume self and return the internal Option<T>
    pub(crate) fn get(self) -> Option<T> {
        self.value
    }

    #[allow(dead_code)]
    fn get_with_tokens(self) -> Option<(TokenStream, T)> {
        match self.value {
            Some(v) => Some((self.tokens, v)),
            None => None,
        }
    }
}

/// ----------------------------------------------------------------------------
/// PBStructAttrs
/// ----------------------------------------------------------------------------

/// Information extracted from the #[pb(...)] attribute on a field:
///   - name: field name
///   - pb_index: `#[pb(index=1)]`
///   - pb_one_of: `#[pb(one_of)]`
///   - skip_pb_serializing / skip_pb_deserializing: `#[pb(skip)]`
///   - serialize_pb_with / deserialize_pb_with: `#[pb(serialize_pb_with="...")]` / `#[pb(deserialize_pb_with="...")]`
pub struct PBStructAttrs {
    pub name: String,
    pb_index: Option<syn::LitInt>,
    pb_one_of: bool,
    skip_pb_serializing: bool,
    skip_pb_deserializing: bool,
    serialize_pb_with: Option<syn::ExprPath>,
    deserialize_pb_with: Option<syn::ExprPath>,
}

impl PBStructAttrs {
    /// Get the #[pb(...)] attribute information from a struct field
    /// and populate PBStructAttrs
    pub fn from_ast(ast_result: &ASTResult, index: usize, field: &syn::Field) -> Self {
        let mut pb_index             = ASTAttr::none(ast_result, PB_INDEX);
        let mut pb_one_of            = BoolAttr::none(ast_result, PB_ONE_OF);
        let mut serialize_pb_with    = ASTAttr::none(ast_result, SERIALIZE_PB_WITH);
        let mut skip_pb_serializing  = BoolAttr::none(ast_result, SKIP_PB_SERIALIZING);
        let mut deserialize_pb_with  = ASTAttr::none(ast_result, DESERIALIZE_PB_WITH);
        let mut skip_pb_deserializing= BoolAttr::none(ast_result, SKIP_PB_DESERIALIZING);

        // If it is a named field, ident.to_string(); otherwise use index
        let ident = match &field.ident {
            Some(id) => id.to_string(),
            None => index.to_string(),
        };

        // Parse #[pb(...)] meta items
        for meta_item in field
            .attrs
            .iter()
            .flat_map(|attr| get_pb_meta_items(ast_result, attr))
            .flatten()
        {
            match &meta_item {
                // Without "=", e.g. #[pb(skip)] / #[pb(one_of)]
                Meta(Path(word)) => {
                    parse_pb_bool_attrs(
                        word,
                        &mut pb_one_of,
                        &mut skip_pb_serializing,
                        &mut skip_pb_deserializing
                    );
                },
                // With "=...", e.g. #[pb(index=1)], #[pb(serialize_pb_with="func")]
                Meta(NameValue(m)) => {
                    parse_pb_keyvalue(
                        ast_result,
                        m,
                        &mut pb_index,
                        &mut serialize_pb_with,
                        &mut deserialize_pb_with
                    );
                },
                // Unknown meta
                Meta(meta_item) => {
                    let path = meta_item.path().into_token_stream().to_string().replace(' ', "");
                    ast_result.error_spanned_by(
                        meta_item.path(),
                        format!("unknown pb field attribute `{}`", path),
                    );
                },
                // Pure literal => error
                Lit(lit_val) => {
                    ast_result.error_spanned_by(lit_val, "unexpected literal in #[pb(...)]");
                },
            }
        }

        PBStructAttrs {
            name: ident,
            pb_index: pb_index.get(),
            pb_one_of: pb_one_of.get(),
            skip_pb_serializing: skip_pb_serializing.get(),
            skip_pb_deserializing: skip_pb_deserializing.get(),
            serialize_pb_with: serialize_pb_with.get(),
            deserialize_pb_with: deserialize_pb_with.get(),
        }
    }

    /// If it is `#[pb(index=123)]`, return "123"
    pub fn pb_index(&self) -> Option<String> {
        self.pb_index.as_ref().map(|lit| lit.base10_digits().to_string())
    }
    /// Whether `#[pb(one_of)]` is added
    pub fn is_one_of(&self) -> bool {
        self.pb_one_of
    }
    /// Whether `#[pb(skip)]` is added (affects serialization)
    pub fn skip_pb_serializing(&self) -> bool {
        self.skip_pb_serializing
    }
    /// Whether `#[pb(skip)]` is added (affects deserialization)
    pub fn skip_pb_deserializing(&self) -> bool {
        self.skip_pb_deserializing
    }
    /// Whether `#[pb(serialize_pb_with="func")]` is written
    pub fn serialize_pb_with(&self) -> Option<&syn::ExprPath> {
        self.serialize_pb_with.as_ref()
    }
    /// Whether `#[pb(deserialize_pb_with="func")]` is written
    pub fn deserialize_pb_with(&self) -> Option<&syn::ExprPath> {
        self.deserialize_pb_with.as_ref()
    }
}

/// Simple wrapper for boolean attributes (such as #[pb(skip)], #[pb(one_of)])
/// set_true(obj) => value=Some(()), otherwise None => false
struct BoolAttr<'c>(ASTAttr<'c, ()>);

impl<'c> BoolAttr<'c> {
    fn none(ast_result: &'c ASTResult, name: Symbol) -> Self {
        BoolAttr(ASTAttr::none(ast_result, name))
    }
    fn set_true<A: ToTokens>(&mut self, obj: A) {
        self.0.set(obj, ());
    }
    fn get(&self) -> bool {
        self.0.value.is_some()
    }
}

/// ---------------------------------------------------------------------------
/// parse_pb_bool_attrs / parse_pb_keyvalue
/// ---------------------------------------------------------------------------

/// Handle attributes like #[pb(skip)] / #[pb(one_of)] that do not have "=value"
fn parse_pb_bool_attrs(
    path: &syn::Path,
    pb_one_of: &mut BoolAttr,
    skip_pb_serializing: &mut BoolAttr,
    skip_pb_deserializing: &mut BoolAttr,
) {
    if path == PB_ONE_OF {
        pb_one_of.set_true(path);
    } else if path == SKIP {
        skip_pb_serializing.set_true(path);
        skip_pb_deserializing.set_true(path);
    }
    // More boolean attributes can be added here in the future, e.g. #[pb(optional)]
}

/// Handle attributes like #[pb(index=...), #[pb(serialize_pb_with="...")], #[pb(deserialize_pb_with="...")]
fn parse_pb_keyvalue(
    ast_result: &ASTResult,
    name_value: &syn::MetaNameValue,

    pb_index: &mut ASTAttr<syn::LitInt>,
    serialize_pb_with: &mut ASTAttr<syn::ExprPath>,
    deserialize_pb_with: &mut ASTAttr<syn::ExprPath>,
) {
    let path = &name_value.path;
    let lit  = &name_value.lit;

    if path == PB_INDEX {
        if let syn::Lit::Int(lit_int) = lit {
            pb_index.set(path, lit_int.clone());
        }
    } else if path == SERIALIZE_PB_WITH {
        if let Ok(path_expr) = parse_lit_into_expr_path(ast_result, SERIALIZE_PB_WITH, lit) {
            serialize_pb_with.set(path, path_expr);
        }
    } else if path == DESERIALIZE_PB_WITH {
        if let Ok(path_expr) = parse_lit_into_expr_path(ast_result, DESERIALIZE_PB_WITH, lit) {
            deserialize_pb_with.set(path, path_expr);
        }
    } else {
        // Unknown key
        let path_str = path.to_token_stream().to_string().replace(' ', "");
        ast_result.error_spanned_by(
            path,
            format!("unknown pb field attribute `{}`", path_str)
        );
    }
}

/// ----------------------------------------------------------------------------
/// is_recognizable_field / is_recognizable_attribute
/// ----------------------------------------------------------------------------

/// Determine whether the current field contains pb/event/node annotations for filtering at a higher level
pub fn is_recognizable_field(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| is_recognizable_attribute(attr))
}

/// If attr.path == PB_ATTRS / EVENT / NODE_ATTRS / NODES_ATTRS, it is considered recognizable
pub fn is_recognizable_attribute(attr: &syn::Attribute) -> bool {
    attr.path == PB_ATTRS || attr.path == EVENT || attr.path == NODE_ATTRS || attr.path == NODES_ATTRS
}

/// ----------------------------------------------------------------------------
/// get_pb_meta_items / get_node_meta_items / get_event_meta_items
/// ----------------------------------------------------------------------------

/// Parse the `#[pb(...)]` attribute and return a Vec<syn::NestedMeta>
pub fn get_pb_meta_items(
    cx: &ASTResult,
    attr: &syn::Attribute,
) -> Result<Vec<syn::NestedMeta>, ()> {
    if attr.path != PB_ATTRS {
        return Ok(vec![]);
    }
    match attr.parse_meta() {
        Ok(List(meta)) => Ok(meta.nested.into_iter().collect()),
        Ok(other) => {
            cx.error_spanned_by(other, "expected #[pb(...)]");
            Err(())
        },
        Err(err) => {
            cx.error_spanned_by(attr, "attribute must be str, e.g. #[pb(xx = \"xxx\")]");
            cx.syn_error(err);
            Err(())
        },
    }
}

/// Parse `#[node(...)]` / `#[nodes(...)]`
pub fn get_node_meta_items(
    cx: &ASTResult,
    attr: &syn::Attribute,
) -> Result<Vec<syn::NestedMeta>, ()> {
    if attr.path != NODE_ATTRS && attr.path != NODES_ATTRS {
        return Ok(vec![]);
    }
    match attr.parse_meta() {
        Ok(List(meta)) => Ok(meta.nested.into_iter().collect()),
        Ok(_) => Ok(vec![]),
        Err(err) => {
            cx.error_spanned_by(attr, "attribute must be str, e.g. #[node(xx = \"...\")]");
            cx.syn_error(err);
            Err(())
        },
    }
}

/// ----------------------------------------------------------------------------
/// parse_lit_into_expr_path / parse_lit_into_ty / parse_lit_str
/// ----------------------------------------------------------------------------

/// Parse a string literal into syn::ExprPath, used for `#[pb(serialize_pb_with="...")]` and similar scenarios
pub fn parse_lit_into_expr_path(
    ast_result: &ASTResult,
    attr_name: Symbol,
    lit: &syn::Lit,
) -> Result<syn::ExprPath, ()> {
    let string = get_lit_str(ast_result, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        ast_result.error_spanned_by(
            lit,
            format!("failed to parse path: {:?}", string.value())
        )
    })
}

/// If lit is not a string, report an error; otherwise return &syn::LitStr
fn get_lit_str<'a>(
    ast_result: &ASTResult,
    attr_name: Symbol,
    lit: &'a syn::Lit,
) -> Result<&'a syn::LitStr, ()> {
    if let syn::Lit::Str(lit_str) = lit {
        Ok(lit_str)
    } else {
        ast_result.error_spanned_by(
            lit,
            format!("expected pb {} attribute to be a string: `{} = \"...\"`", attr_name, attr_name),
        );
        Err(())
    }
}

/// Parse a string literal (syn::LitStr) into syn::Type or syn::ExprPath
fn parse_lit_into_ty(
    ast_result: &ASTResult,
    attr_name: Symbol,
    lit: &syn::Lit,
) -> Result<syn::Type, ()> {
    let string = get_lit_str(ast_result, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        ast_result.error_spanned_by(
            lit,
            format!("failed to parse type: {} = {:?}", attr_name, string.value())
        )
    })
}

/// General string literal -> T parsing (T: Parse), can be syn::Type or syn::ExprPath
pub fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where
    T: Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

/// ----------------------------------------------------------------------------
/// spanned_tokens / respan_token_stream
/// ----------------------------------------------------------------------------

/// To maintain correct span information, parse the content of LitStr into a TokenStream
fn spanned_tokens(s: &syn::LitStr) -> parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan_token_stream(stream, s.span()))
}

/// Recursively replace the token's span with s.span() to maintain line and column information
fn respan_token_stream(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token_tree(token, span))
        .collect()
}

fn respan_token_tree(mut token: TokenTree, span: Span) -> TokenTree {
    if let TokenTree::Group(g) = &mut token {
        *g = Group::new(g.delimiter(), respan_token_stream(g.stream(), span));
    }
    token.set_span(span);
    token
    // This way, the parsed token can have correct line and column information when an error occurs
}

/// ----------------------------------------------------------------------------
/// default_pb_type
/// ----------------------------------------------------------------------------

/// If #[pb(struct="Type")] is not explicitly specified, use Rust's ident as the proto type
/// e.g. struct Foo => message Foo { ... }
fn default_pb_type(ast_result: &ASTResult, ident: &syn::Ident) -> syn::Type {
    let take_ident = ident.to_string();
    let lit_str = syn::LitStr::new(&take_ident, ident.span());
    if let Ok(tokens) = spanned_tokens(&lit_str) {
        if let Ok(pb_struct_ty) = syn::parse2(tokens) {
            return pb_struct_ty;
        }
    }
    ast_result.error_spanned_by(
        ident,
        format!("❌ Can't find {} protobuf struct", take_ident),
    );
    panic!()
}

/// ----------------------------------------------------------------------------
/// is_option / ungroup
/// ----------------------------------------------------------------------------

#[allow(dead_code)]
pub fn is_option(ty: &syn::Type) -> bool {
    let path = match ungroup(ty) {
        syn::Type::Path(ty) => &ty.path,
        _ => {
            return false;
        },
    };
    let seg = match path.segments.last() {
        Some(seg) => seg,
        None => {
            return false;
        },
    };
    let args = match &seg.arguments {
        syn::PathArguments::AngleBracketed(bracketed) => &bracketed.args,
        _ => {
            return false;
        },
    };
    seg.ident == "Option" && args.len() == 1
}

#[allow(dead_code)]
pub fn ungroup(mut ty: &syn::Type) -> &syn::Type {
    while let syn::Type::Group(group) = ty {
        ty = &group.elem;
    }
    ty
}

#[cfg(test)]
mod tests {
    use super::*; // import PBAttrsContainer, parse_lit_into_ty, etc
    use quote::ToTokens;
    use syn::{parse_quote, DeriveInput};

    use crate::ctxt::ASTResult; // 如果 ASTResult 定义在另一个mod

    #[test]
    fn test_pbattrscontainer_for_struct() {
        // 1) 构造一个带 #[pb(struct="MyProto")] 的struct
        let input: DeriveInput = parse_quote! {
            #[pb(struct="MyProto")]
            struct Demo {
                foo: i32
            }
        };
        let ast_result = ASTResult::new();

        // 2) 调用 from_ast
        let container = PBAttrsContainer::from_ast(&ast_result, &input);
        // 3) container.pb_struct_type() 应当是 Some(syn::Type), 其 to_token_stream() => "MyProto"
        let pb_ty = container.pb_struct_type().expect("Should have struct type");
        assert_eq!(pb_ty.to_token_stream().to_string(), "MyProto");

        // 4) pb_enum_type 为空
        assert!(container.pb_enum_type().is_none());

        // 5) 不应有错误
        ast_result.check().unwrap();
    }

    #[test]
    fn test_pbattrscontainer_for_enum() {
        // 1) 构造一个带 #[pb(enum="SomeEnum")] 的 enum
        let input: DeriveInput = parse_quote! {
            #[pb(enum="SomeEnum")]
            enum MyE { A, B }
        };
        let ast_result = ASTResult::new();

        let container = PBAttrsContainer::from_ast(&ast_result, &input);
        let pb_enum_ty = container.pb_enum_type().expect("Should have enum type");
        assert_eq!(pb_enum_ty.to_token_stream().to_string(), "SomeEnum");

        // struct type 应当是 none
        assert!(container.pb_struct_type().is_none());
        ast_result.check().unwrap();
    }

    #[test]
    fn test_pbattrscontainer_unknown_attr() {
        // 1) 带 #[pb(unknown="Foo")] => 解析时会 error_spanned_by
        let input: DeriveInput = parse_quote! {
            #[pb(unknown="Foo")]
            struct Bad {
                foo: i32,
            }
        };
        let ast_result = ASTResult::new();
        let _ = PBAttrsContainer::from_ast(&ast_result, &input);

        // 2) check() 应当有错误
        match ast_result.check() {
            Ok(_) => panic!("Expected error for unknown pb container attribute"),
            Err(errs) => {
                assert!(!errs.is_empty());
                assert!(errs[0].to_string().contains("unknown container attribute `unknown`"));
            }
        }
    }
}