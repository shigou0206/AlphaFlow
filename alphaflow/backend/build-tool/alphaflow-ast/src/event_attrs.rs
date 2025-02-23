#![allow(clippy::all)]

use crate::{
    symbol::*,
    // ASTResult 定义在 ctxt.rs 中，所以从 crate::ctxt 导入
    ctxt::ASTResult
};
// use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    self,
    Meta::{NameValue, Path},
    NestedMeta::{Lit, Meta},
};

/// Contains event attribute information, such as input, output, error_ty, ignore.
#[derive(Debug, Clone)]
pub struct EventAttrs {
    pub input: Option<syn::Path>,
    pub output: Option<syn::Path>,
    pub error_ty: Option<String>,
    pub ignore: bool,
}

/// Stores context information required for "enum variant events"
#[derive(Debug, Clone)]
pub struct EventEnumAttrs {
    pub enum_name: String,
    pub enum_item_name: String,
    /// integer discriminant as string (e.g. "1", "2"), or "" if none
    pub value: String,
    pub event_attrs: EventAttrs,
}

impl EventEnumAttrs {
    /// Extracts event annotations from an enum variant
    pub fn from_ast(
        ast_result: &ASTResult,
        ident: &syn::Ident,
        variant: &syn::Variant,
        enum_attrs: &[syn::Attribute],
    ) -> Self {
        // 1) variant & enum name
        let enum_item_name = variant.ident.to_string();
        let enum_name = ident.to_string();

        // 2) If there's a discriminant => only integer literal is accepted
        let mut value = String::new();
        if let Some((_eq_token, expr)) = variant.discriminant.as_ref() {
            if let syn::Expr::Lit(expr_lit) = expr {
                if let syn::Lit::Int(int_value) = &expr_lit.lit {
                    value = int_value.base10_digits().to_string();
                } else {
                    ast_result.push_spanned(
                        &expr_lit.lit,
                        "Only integer discriminant is supported for event variant"
                    );
                }
            }
        }

        // 3) Combine enum-level attribute (#[event_err="..."]) & variant-level #[event(...)]
        let event_attrs = get_event_attrs_from(ast_result, &variant.attrs, enum_attrs);

        EventEnumAttrs {
            enum_name,
            enum_item_name,
            value,
            event_attrs,
        }
    }

    pub fn event_input(&self) -> Option<syn::Path> {
        self.event_attrs.input.clone()
    }
    pub fn event_output(&self) -> Option<syn::Path> {
        self.event_attrs.output.clone()
    }
    pub fn event_error(&self) -> String {
        // original logic: unwrap
        self.event_attrs.error_ty.as_ref().unwrap().clone()
    }
}

/// merges enum-level #[event_err="..."] and variant-level #[event(...)]
fn get_event_attrs_from(
    ast_result: &ASTResult,
    variant_attrs: &[syn::Attribute],
    enum_attrs: &[syn::Attribute],
) -> EventAttrs {
    let mut ea = EventAttrs {
        input: None,
        output: None,
        error_ty: None,
        ignore: false,
    };

    // parse enum-level #[event_err="..."]
    for attr in enum_attrs {
        if attr.path.segments.iter().any(|seg| seg.ident == EVENT_ERR) {
            match attr.parse_meta() {
                Ok(NameValue(named_value)) => {
                    if let syn::Lit::Str(s) = named_value.lit {
                        ea.error_ty = Some(s.value());
                    } else {
                        ast_result.push_spanned(
                            &named_value.lit,
                            format!(
                                "#[event_err] must be string literal, found: {:?}",
                                named_value.lit
                            )
                        );
                    }
                },
                Err(e) => {
                    ast_result.push_error(e);
                },
                _ => {
                    ast_result.push_spanned(
                        attr,
                        "Cannot parse #[event_err=\"...\"] from this attribute"
                    );
                }
            }
        }
    }

    // parse variant-level #[event(...)]
    let attr_meta_items_info = variant_attrs
        .iter()
        .flat_map(|attr| match get_event_meta_items(ast_result, attr) {
            Ok(items) => Some((attr, items)),
            Err(_) => None,
        })
        .collect::<Vec<(&syn::Attribute, Vec<syn::NestedMeta>)>>();

    for (attr, nested_metas) in attr_meta_items_info {
        for meta_item in &nested_metas {
            match meta_item {
                // e.g. #[event(input="foo")]
                Meta(NameValue(name_value)) => {
                    parse_event_keyvalue(ast_result, attr, name_value, &mut ea);
                }
                // e.g. #[event(ignore)]
                Meta(Path(path)) => {
                    if path == EVENT_IGNORE && attr.path == EVENT {
                        ea.ignore = true;
                    }
                }
                Lit(lit_val) => {
                    ast_result.push_spanned(lit_val, "Unexpected literal in #[event(...)]");
                }
                _ => {
                    ast_result.push_spanned(
                        meta_item,
                        "Unexpected item in #[event(...)]"
                    );
                }
            }
        }
    }

    ea
}

/// parse key-value pairs in `#[event(key="...")]` => e.g. [event(input="...")] or [event(output="...")]
fn parse_event_keyvalue(
    ast_result: &ASTResult,
    _attr: &syn::Attribute,
    name_value: &syn::MetaNameValue,
    event_attrs: &mut EventAttrs,
) {
    let path = &name_value.path;
    // must be string literal
    let str_lit = match &name_value.lit {
        syn::Lit::Str(s) => s,
        other => {
            ast_result.push_spanned(
                &name_value.lit,
                format!("Expected string literal, found: {:?}", other)
            );
            return;
        }
    };

    if path == EVENT_INPUT {
        // parse input path => e.g. "req::Foo"
        match parse_lit_str(str_lit) {
            Ok(p) => event_attrs.input = Some(p),
            Err(_) => {
                ast_result.push_spanned(
                    str_lit,
                    format!("Failed to parse request path: {:?}", str_lit.value())
                );
            }
        }
    } else if path == EVENT_OUTPUT {
        // parse output path => e.g. "rsp::Bar"
        match parse_lit_str(str_lit) {
            Ok(p) => event_attrs.output = Some(p),
            Err(_) => {
                ast_result.push_spanned(
                    str_lit,
                    format!("Failed to parse response path: {:?}", str_lit.value())
                );
            }
        }
    } else {
        // unknown key => error
        ast_result.push_spanned(
            path,
            format!("Unknown key in #[event(...)] : {:?}", path.to_token_stream())
        );
    }
}

/// parse `#[event(...)]`
pub fn get_event_meta_items(
    cx: &ASTResult,
    attr: &syn::Attribute,
) -> Result<Vec<syn::NestedMeta>, ()> {
    if attr.path != EVENT {
        return Ok(vec![]);
    }

    match attr.parse_meta() {
        Ok(syn::Meta::List(meta)) => Ok(meta.nested.into_iter().collect()),
        Ok(other) => {
            cx.push_spanned(other, "expected #[event(...)]");
            Err(())
        },
        Err(err) => {
            cx.push_spanned(
                attr,
                "attribute must be str, e.g. #[event(xx = \"...\")]"
            );
            cx.syn_error(err);
            Err(())
        },
    }
}

/// parse string literal => syn::Path
fn parse_lit_str<T>(s: &syn::LitStr) -> syn::parse::Result<T>
where
    T: syn::parse::Parse,
{
    let tokens = syn::parse_str(&s.value())?;
    syn::parse2(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, ItemEnum};
    // ASTResult 来自 crate::ctxt
    use crate::ctxt::ASTResult;

    #[test]
    fn test_event_enum_attrs_basic() {
        let item_enum: ItemEnum = parse_quote! {
            #[event_err="MyErr"]
            enum MyEvent {
                #[event(input="req::Foo", output="rsp::Bar")]
                A = 1,
                #[event(ignore)]
                B,
            }
        };

        let ast_result = ASTResult::new();
        let enum_attrs = &item_enum.attrs;

        let variants = &item_enum.variants;
        // variant A
        let e_a = EventEnumAttrs::from_ast(&ast_result, &item_enum.ident, &variants[0], enum_attrs);
        assert_eq!(e_a.enum_name, "MyEvent");
        assert_eq!(e_a.enum_item_name, "A");
        assert_eq!(e_a.value, "1");

        // event_err => "MyErr"
        let err_ty = e_a.event_error();
        assert_eq!(err_ty, "MyErr");

        // input/output => Some("req :: Foo"), Some("rsp :: Bar")
        let in_path = e_a.event_input().unwrap();
        assert_eq!(in_path.to_token_stream().to_string(), "req :: Foo");
        let out_path = e_a.event_output().unwrap();
        assert_eq!(out_path.to_token_stream().to_string(), "rsp :: Bar");

        // variant B
        let e_b = EventEnumAttrs::from_ast(&ast_result, &item_enum.ident, &variants[1], enum_attrs);
        assert_eq!(e_b.enum_item_name, "B");
        // no discriminant => ""
        assert_eq!(e_b.value, "");
        // event(ignore) => e_b.event_attrs.ignore=true
        assert!(e_b.event_attrs.ignore);

        // no errors
        ast_result.check().unwrap();
    }

    #[test]
    fn test_event_enum_attrs_non_int_discriminant() {
        let item_enum: ItemEnum = parse_quote! {
            enum MyEvent {
                #[event]
                A = "not int",
            }
        };
        let ast_result = ASTResult::new();
        let enum_attrs = &item_enum.attrs;

        let e_a = EventEnumAttrs::from_ast(&ast_result, &item_enum.ident, &item_enum.variants[0], enum_attrs);
        // not int => e_a.value => ""
        assert_eq!(e_a.value, "");

        // expect 1 error
        match ast_result.check() {
            Ok(()) => panic!("expected error for non-int discriminant"),
            Err(errs) => {
                assert_eq!(errs.len(), 1, "Expected exactly 1 error");
                let msg = errs[0].to_string();
                assert!(msg.contains("Only integer discriminant is supported for event variant"));
            }
        }
    }

    #[test]
    fn test_event_unknown_key() {
        let item_enum: ItemEnum = parse_quote! {
            enum MyEvent {
                #[event(foo="???")]
                A = 2,
            }
        };
        let ast_result = ASTResult::new();
        let enum_attrs = &item_enum.attrs;

        let e_a = EventEnumAttrs::from_ast(&ast_result, &item_enum.ident, &item_enum.variants[0], enum_attrs);
        assert_eq!(e_a.value, "2");

        // expect error
        match ast_result.check() {
            Ok(()) => panic!("expected error for unknown [event(foo=...)]"),
            Err(errs) => {
                assert!(!errs.is_empty());
                let msg = errs[0].to_string();
                assert!(msg.contains("Unknown key in #[event(...)] :"));
            }
        }
    }
}