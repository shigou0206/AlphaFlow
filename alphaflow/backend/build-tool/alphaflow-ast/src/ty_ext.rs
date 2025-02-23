use crate::ctxt::ASTResult;
use syn::{self, AngleBracketedGenericArguments, PathSegment};

/// ----------------------------------------------------------------------------
/// PrimitiveTy / MapInfo
/// ----------------------------------------------------------------------------

#[derive(Eq, PartialEq, Debug)]
pub enum PrimitiveTy {
    Map(MapInfo),
    Vec,
    Opt,
    Other,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MapInfo {
    pub key: String,
    pub value: String,
}

impl MapInfo {
    fn new(key: String, value: String) -> Self {
        MapInfo { key, value }
    }
}

/// ----------------------------------------------------------------------------
/// TyInfo
/// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct TyInfo<'a> {
    pub ident: &'a syn::Ident,
    pub ty: &'a syn::Type,
    pub primitive_ty: PrimitiveTy,
    pub bracket_ty_info: Box<Option<TyInfo<'a>>>,
}

impl<'a> TyInfo<'a> {
    #[allow(dead_code)]
    pub fn bracketed_ident(&'a self) -> &'a syn::Ident {
        match self.bracket_ty_info.as_ref() {
            Some(b_ty) => b_ty.ident,
            None => panic!("called bracketed_ident on a non-bracket type"),
        }
    }
}

/// ----------------------------------------------------------------------------
/// parse_ty (core entry point)
/// ----------------------------------------------------------------------------

pub fn parse_ty<'a>(
    ast_result: &ASTResult,
    ty: &'a syn::Type,
) -> Result<Option<TyInfo<'a>>, String> {
    if let syn::Type::Path(ref p) = ty {
        // multi-segment path => not recognized => Ok(None)
        if p.path.segments.len() != 1 {
            return Ok(None);
        }
        let seg = match p.path.segments.last() {
            Some(seg) => seg,
            None => return Ok(None),
        };

        if let syn::PathArguments::AngleBracketed(ref bracketed) = seg.arguments {
            match seg.ident.to_string().as_ref() {
                "HashMap" => {
                    return generate_hashmap_ty_info(ast_result, ty, seg, bracketed);
                }
                "Vec" => {
                    return generate_vec_ty_info(ast_result, seg, bracketed);
                }
                "Option" => {
                    return generate_option_ty_info(ast_result, ty, seg, bracketed);
                }
                _ => {
                    let msg = format!("Unsupported type with generics: {}", seg.ident);
                    ast_result.error_spanned_by(&seg.ident, &msg);
                    return Err(msg);
                }
            }
        } else {
            // no generics => treat as Other
            return Ok(Some(TyInfo {
                ident: &seg.ident,
                ty,
                primitive_ty: PrimitiveTy::Other,
                bracket_ty_info: Box::new(None),
            }));
        }
    }
    Err("Unsupported inner type, get inner type fail".to_string())
}

/// ----------------------------------------------------------------------------
/// parse_bracketed
/// ----------------------------------------------------------------------------

fn parse_bracketed(bracketed: &AngleBracketedGenericArguments) -> Vec<&syn::Type> {
    bracketed
        .args
        .iter()
        .flat_map(|arg| {
            if let syn::GenericArgument::Type(ref ty_in_bracket) = arg {
                Some(ty_in_bracket)
            } else {
                None
            }
        })
        .collect::<Vec<&syn::Type>>()
}

/// ----------------------------------------------------------------------------
/// generate_*_ty_info
/// ----------------------------------------------------------------------------

pub fn generate_hashmap_ty_info<'a>(
    ast_result: &ASTResult,
    ty: &'a syn::Type,
    path_segment: &'a PathSegment,
    bracketed: &'a AngleBracketedGenericArguments,
) -> Result<Option<TyInfo<'a>>, String> {
    if bracketed.args.len() != 2 {
        return Ok(None);
    }
    let types = parse_bracketed(bracketed);
    // parse K
    let key_tyinfo = parse_ty(ast_result, types[0])?;
    let key_str = match key_tyinfo {
        Some(ref info) => info.ident.to_string(),
        None => return Ok(None),
    };
    // parse V
    let val_tyinfo = parse_ty(ast_result, types[1])?;
    let val_str = match val_tyinfo {
        Some(ref info) => info.ident.to_string(),
        None => return Ok(None),
    };

    Ok(Some(TyInfo {
        ident: &path_segment.ident,
        ty,
        primitive_ty: PrimitiveTy::Map(MapInfo::new(key_str, val_str)),
        bracket_ty_info: Box::new(val_tyinfo),
    }))
}

fn generate_option_ty_info<'a>(
    ast_result: &ASTResult,
    ty: &'a syn::Type,
    path_segment: &'a PathSegment,
    bracketed: &'a AngleBracketedGenericArguments,
) -> Result<Option<TyInfo<'a>>, String> {
    assert_eq!(path_segment.ident.to_string(), "Option");
    let types = parse_bracketed(bracketed);
    if types.len() != 1 {
        return Ok(None);
    }
    let bracket_ty_info = Box::new(parse_ty(ast_result, types[0])?);

    Ok(Some(TyInfo {
        ident: &path_segment.ident,
        ty,
        primitive_ty: PrimitiveTy::Opt,
        bracket_ty_info,
    }))
}

fn generate_vec_ty_info<'a>(
    ast_result: &ASTResult,
    path_segment: &'a PathSegment,
    bracketed: &'a AngleBracketedGenericArguments,
) -> Result<Option<TyInfo<'a>>, String> {
    if bracketed.args.len() != 1 {
        return Ok(None);
    }
    if let syn::GenericArgument::Type(ref bracketed_type) = bracketed.args.first().unwrap() {
        let bracketed_ty_info = Box::new(parse_ty(ast_result, bracketed_type)?);
        return Ok(Some(TyInfo {
            ident: &path_segment.ident,
            ty: bracketed_type,
            primitive_ty: PrimitiveTy::Vec,
            bracket_ty_info: bracketed_ty_info,
        }));
    }
    Ok(None)
}

/// ----------------------------------------------------------------------------
/// Tests
/// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    use syn::Type;

    // 如果 ASTResult 在同 crate 下的另一个模块定义，需要在lib.rs或mod层声明一下
    // 并在此 use crate::ctxt::ASTResult;
    use crate::ctxt::ASTResult;

    #[test]
    fn test_parse_ty_simple() {
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(i32);

        let parsed = parse_ty(&ast_result, &t).unwrap();
        assert!(parsed.is_some());
        let ty_info = parsed.unwrap();
        assert_eq!(ty_info.primitive_ty, PrimitiveTy::Other); 
        assert_eq!(ty_info.ident.to_string(), "i32");
        assert!(ty_info.bracket_ty_info.is_none());

        ast_result.check().unwrap();
    }

    #[test]
    fn test_parse_ty_string() {
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(String);

        let parsed = parse_ty(&ast_result, &t).unwrap();
        assert!(parsed.is_some());
        let ty_info = parsed.unwrap();
        assert_eq!(ty_info.primitive_ty, PrimitiveTy::Other);
        assert_eq!(ty_info.ident.to_string(), "String");
        assert!(ty_info.bracket_ty_info.is_none());

        ast_result.check().unwrap();
    }

    #[test]
    fn test_parse_ty_vec() {
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(Vec<String>);

        let parsed = parse_ty(&ast_result, &t).unwrap();
        assert!(parsed.is_some());
        let ty_info = parsed.unwrap();
        assert_eq!(ty_info.primitive_ty, PrimitiveTy::Vec);
        assert_eq!(ty_info.ident.to_string(), "Vec");

        // bracket_ty_info
        assert!(ty_info.bracket_ty_info.is_some());
        let inner_info = ty_info.bracket_ty_info.as_ref().as_ref().unwrap(); 
        // ↑ 修正：.as_ref().as_ref().unwrap() => &TyInfo

        assert_eq!(inner_info.ident.to_string(), "String");
        assert_eq!(inner_info.primitive_ty, PrimitiveTy::Other);

        ast_result.check().unwrap();
    }

    #[test]
    fn test_parse_ty_option() {
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(Option<i32>);

        let parsed = parse_ty(&ast_result, &t).unwrap();
        assert!(parsed.is_some());
        let ty_info = parsed.unwrap();
        assert_eq!(ty_info.primitive_ty, PrimitiveTy::Opt);
        assert_eq!(ty_info.ident.to_string(), "Option");

        // bracket_ty_info
        let inner_info = ty_info.bracket_ty_info.as_ref().as_ref().unwrap(); 
        // ↑ 同样修正
        assert_eq!(inner_info.ident.to_string(), "i32");
        assert_eq!(inner_info.primitive_ty, PrimitiveTy::Other);

        ast_result.check().unwrap();
    }

    #[test]
    fn test_parse_ty_hashmap() {
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(HashMap<String, i32>);

        let parsed = parse_ty(&ast_result, &t).unwrap();
        assert!(parsed.is_some());
        let ty_info = parsed.unwrap();

        match &ty_info.primitive_ty {
            PrimitiveTy::Map(map_info) => {
                assert_eq!(map_info.key, "String");
                assert_eq!(map_info.value, "i32");
            },
            _ => panic!("expected PrimitiveTy::Map"),
        }
        assert_eq!(ty_info.ident.to_string(), "HashMap");

        ast_result.check().unwrap();
    }

    #[test]
    fn test_parse_ty_multi_segment() {
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(std::vec::Vec<u8>);

        let parsed = parse_ty(&ast_result, &t).unwrap();
        // parse_ty => p.path.segments.len()!=1 => return Ok(None)
        assert!(parsed.is_none());

        ast_result.check().unwrap();
    }

    #[test]
    fn test_parse_ty_unsupported_generic() {
        // "Foo<Bar>" => not HashMap/Vec/Option => error
        let ast_result = ASTResult::new();
        let t: Type = parse_quote!(Foo<Bar>);

        let parsed = parse_ty(&ast_result, &t);
        assert!(parsed.is_err(), "Should return Err for unsupported generic type Foo<Bar>");

        match ast_result.check() {
            Ok(()) => panic!("Expected error from unsupported type"),
            Err(errors) => {
                assert!(!errors.is_empty());
                let msg = errors[0].to_string();
                assert!(msg.contains("Unsupported type with generics: Foo"));
            }
        }
    }
}