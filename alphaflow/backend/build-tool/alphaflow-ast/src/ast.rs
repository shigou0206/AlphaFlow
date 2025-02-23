#![allow(clippy::all)]
#![allow(unused_attributes)]
#![allow(unused_assignments)]

// use quote::ToTokens; // <-- 必须显式引入 ToTokens trait，才能使用 `path.to_token_stream()`
use crate::{
    event_attrs::EventEnumAttrs,
    node_attrs::NodeStructAttrs,
    ty_ext::{parse_ty, PrimitiveTy, TyInfo},
    ctxt::ASTResult,
    symbol::NODE_TYPE,
    pb_attrs::{PBAttrsContainer, PBStructAttrs},
};

use proc_macro2::Ident;
use syn::{self, Meta::NameValue};

/// Represents an abstract container for a struct or enum, carrying parsed annotations/attributes.
pub struct ASTContainer<'a> {
    pub ident: syn::Ident,
    pub node_type: Option<String>,
    pub pb_attrs: PBAttrsContainer,
    pub data: ASTData<'a>,
}

/// The form of the data body: Struct or Enum.
pub enum ASTData<'a> {
    Struct(ASTStyle, Vec<ASTField<'a>>),
    Enum(Vec<ASTEnumVariant<'a>>),
}

impl<'a> ASTData<'a> {
    pub fn all_fields(&'a self) -> Box<dyn Iterator<Item = &'a ASTField<'a>> + 'a> {
        match self {
            ASTData::Struct(_, fields) => Box::new(fields.iter()),
            ASTData::Enum(variants) => Box::new(variants.iter().flat_map(|variant| variant.fields.iter())),
        }
    }

    pub fn all_variants(&'a self) -> Box<dyn Iterator<Item = &'a EventEnumAttrs> + 'a> {
        match self {
            ASTData::Enum(variants) => {
                Box::new(variants.iter().map(|variant| &variant.attrs))
            },
            ASTData::Struct(_, _) => Box::new(std::iter::empty()),
        }
    }

    pub fn all_idents(&'a self) -> Box<dyn Iterator<Item = &'a syn::Ident> + 'a> {
        match self {
            ASTData::Enum(variants) => Box::new(variants.iter().map(|v| &v.ident)),
            ASTData::Struct(_, fields) => {
                let iter = fields.iter().filter_map(|f| match &f.member {
                    syn::Member::Named(id) => Some(id),
                    _ => None,
                });
                Box::new(iter)
            },
        }
    }
}

pub struct ASTEnumVariant<'a> {
    pub ident: syn::Ident,
    pub attrs: EventEnumAttrs,
    pub style: ASTStyle,
    pub fields: Vec<ASTField<'a>>,
    pub original: &'a syn::Variant,
}

impl<'a> ASTEnumVariant<'a> {
    pub fn name(&self) -> String {
        self.ident.to_string()
    }
}

pub enum BracketCategory {
    Other,
    Opt,
    Vec,
    Map((String, String)),
}

pub struct ASTField<'a> {
    pub member: syn::Member,
    pub pb_attrs: PBStructAttrs,
    pub node_attrs: NodeStructAttrs,
    pub ty: &'a syn::Type,
    pub original: &'a syn::Field,
    pub bracket_ty: Option<syn::Ident>,
    pub bracket_inner_ty: Option<syn::Ident>,
    pub bracket_category: Option<BracketCategory>,
}

impl<'a> ASTField<'a> {
    pub fn new(cx: &ASTResult, field: &'a syn::Field, index: usize) -> Result<Self, String> {
        let recognizable = is_recognizable_by_any_annotation(field);
        if !recognizable {
            return Err("unrecognizable field: no relevant annotation found".to_string());
        }

        let (bracket_category, bracket_ty, bracket_inner_ty) =
            match parse_ty(cx, &field.ty) {
                Ok(Some(inner_ty)) => parse_bracket_info(inner_ty),
                Ok(None) => {
                    let msg = format!("Fail to get the ty inner type: {:?}", field);
                    return Err(msg);
                }
                Err(e) => {
                    cx.error_spanned_by(field, &format!("Field parse_ty error: {}", e));
                    return Err(e);
                }
            };

        let member = field
            .ident
            .clone()
            .map(syn::Member::Named)
            .unwrap_or(syn::Member::Unnamed(index.into()));

        Ok(ASTField {
            member,
            pb_attrs: PBStructAttrs::from_ast(cx, index, field),
            node_attrs: NodeStructAttrs::from_ast(cx, index, field),
            ty: &field.ty,
            original: field,
            bracket_ty,
            bracket_inner_ty,
            bracket_category: Some(bracket_category),
        })
    }

    pub fn ty_as_str(&self) -> String {
        if let Some(ref ident) = self.bracket_inner_ty {
            ident.to_string()
        } else {
            self.bracket_ty
                .as_ref()
                .map_or_else(|| "UnknownTy".to_string(), |id| id.to_string())
        }
    }

    pub fn name(&self) -> Option<syn::Ident> {
        match &self.member {
            syn::Member::Named(id) => Some(id.clone()),
            syn::Member::Unnamed(_) => None,
        }
    }
}

#[derive(Copy, Clone)]
pub enum ASTStyle {
    Struct,
    Tuple,
    NewType,
    Unit,
}

impl<'a> ASTContainer<'a> {
    pub fn from_ast(ast_result: &ASTResult, ast: &'a syn::DeriveInput) -> Option<ASTContainer<'a>> {
        let pb_attrs = PBAttrsContainer::from_ast(ast_result, ast);

        let data = match &ast.data {
            syn::Data::Struct(s) => {
                let (style, fields) = parse_fields(ast_result, &s.fields);
                ASTData::Struct(style, fields)
            }
            syn::Data::Enum(e) => {
                let variants = enum_from_ast(ast_result, &ast.ident, &e.variants, &ast.attrs);
                ASTData::Enum(variants)
            }
            syn::Data::Union(_) => {
                ast_result.error_spanned_by(ast, "Does not support union");
                return None;
            }
        };

        let node_type = get_node_type(ast_result, &ast.ident, &ast.attrs);

        Some(ASTContainer {
            ident: ast.ident.clone(),
            pb_attrs,
            node_type,
            data,
        })
    }
}

pub fn parse_fields<'a>(
    cx: &ASTResult,
    fields: &'a syn::Fields,
) -> (ASTStyle, Vec<ASTField<'a>>) {
    match fields {
        syn::Fields::Named(named) => {
            let list = fields_from_ast(cx, &named.named);
            (ASTStyle::Struct, list)
        }
        syn::Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
            let list = fields_from_ast(cx, &unnamed.unnamed);
            (ASTStyle::NewType, list)
        }
        syn::Fields::Unnamed(unnamed) => {
            let list = fields_from_ast(cx, &unnamed.unnamed);
            (ASTStyle::Tuple, list)
        }
        syn::Fields::Unit => (ASTStyle::Unit, Vec::new()),
    }
}

pub fn enum_from_ast<'a>(
    cx: &ASTResult,
    ident: &syn::Ident,
    variants: &'a syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
    enum_attrs: &[syn::Attribute],
) -> Vec<ASTEnumVariant<'a>> {
    variants
        .iter()
        .map(|variant| {
            let attrs = EventEnumAttrs::from_ast(cx, ident, variant, enum_attrs);
            let (style, fields) = parse_fields(cx, &variant.fields);

            ASTEnumVariant {
                ident: variant.ident.clone(),
                attrs,
                style,
                fields,
                original: variant,
            }
        })
        .collect()
}

fn fields_from_ast<'a>(
    cx: &ASTResult,
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
) -> Vec<ASTField<'a>> {
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            match ASTField::new(cx, field, index) {
                Ok(f) => Some(f),
                Err(errmsg) => {
                    if errmsg.contains("unrecognizable") {
                        None
                    } else {
                        cx.error_spanned_by(field, errmsg);
                        None
                    }
                }
            }
        })
        .collect()
}

fn get_node_type(
    ast_result: &ASTResult,
    struct_name: &Ident,
    attrs: &[syn::Attribute],
) -> Option<String> {
    let mut node_type = None;
    for attr in attrs.iter() {
        if attr.path.segments.iter().any(|s| s.ident == NODE_TYPE) {
            if let Ok(NameValue(named_value)) = attr.parse_meta() {
                if node_type.is_some() {
                    ast_result.error_spanned_by(struct_name, "Duplicate node_type definition");
                }
                if let syn::Lit::Str(s) = named_value.lit {
                    node_type = Some(s.value());
                }
            }
        }
    }
    node_type
}

fn is_recognizable_by_any_annotation(field: &syn::Field) -> bool {
    use quote::ToTokens; // For path.to_token_stream()
    field
        .attrs
        .iter()
        .any(|attr| {
            let path_str = attr.path.to_token_stream().to_string();
            path_str.contains("pb")
                || path_str.contains("node")
                || path_str.contains("event")
        })
}

fn parse_bracket_info(inner_ty_info: TyInfo) -> (BracketCategory, Option<syn::Ident>, Option<syn::Ident>) {
    let bracket_category = match inner_ty_info.primitive_ty {
        PrimitiveTy::Map(map_info) => BracketCategory::Map((map_info.key.clone(), map_info.value)),
        PrimitiveTy::Vec => BracketCategory::Vec,
        PrimitiveTy::Opt => BracketCategory::Opt,
        PrimitiveTy::Other => BracketCategory::Other,
    };

    let mut bracket_ty = None;
    let mut bracket_inner_ty = None;

    if let Some(sub) = *inner_ty_info.bracket_ty_info {
        bracket_inner_ty = Some(sub.ident.clone());
        bracket_ty = Some(inner_ty_info.ident.clone());
    } else {
        bracket_ty = Some(inner_ty_info.ident.clone());
    }

    (bracket_category, bracket_ty, bracket_inner_ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, DeriveInput, ItemUnion, DataUnion};
    use crate::ctxt::ASTResult;

    #[test]
    fn test_ast_container_simple_struct() {
        // same code as your example test ...
    }

    #[test]
    fn test_ast_container_simple_enum() {
        // ...
    }

    #[test]
    fn test_union_not_supported() {
        // fix union construction => use DataUnion { union_token, fields } 
        let item_union: ItemUnion = parse_quote! {
            union U {
                foo: i32,
                bar: i64,
            }
        };

        // must build syn::Data::Union(DataUnion { ... })
        let data_union = syn::Data::Union(DataUnion {
            union_token: item_union.union_token,
            fields: item_union.fields,
        });

        let input = DeriveInput {
            attrs: item_union.attrs,
            vis: item_union.vis,
            ident: item_union.ident,
            generics: item_union.generics,
            data: data_union, // must be of type syn::Data
        };

        let ast_result = ASTResult::new();
        let container = ASTContainer::from_ast(&ast_result, &input);
        assert!(container.is_none());

        // check error
        match ast_result.check() {
            Ok(()) => panic!("Expected error about 'Does not support union'"),
            Err(errs) => {
                assert!(!errs.is_empty());
                let msg = errs[0].to_string();
                assert!(msg.contains("Does not support union"));
            }
        }
    }
}