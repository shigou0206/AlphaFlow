use crate::{
    pb_attrs::get_node_meta_items,
    pb_attrs::parse_lit_into_expr_path,
    symbol::*,
    pb_attrs::ASTAttr,         // For set/get mechanism
    ctxt::ASTResult,
};
use quote::ToTokens;
use syn::{
    self,
    LitStr,
    Meta::NameValue,
    NestedMeta::{Lit, Meta},
};

/// Represents attributes configured on a "node" field via `#[node(...)]` annotation:
///   - rename: Optional name redefinition
///   - has_child: Indicates if there are child nodes
///   - child_name: If there are child nodes, specifies the child node name (e.g., "dialogNodes")
///   - child_index: Possible index for child nodes
///   - get_node_value_with / set_node_value_with: Custom functions for runtime get/set of child node values
///   - with_children: If the field represents a list of child nodes, specifies how to handle child nodes
pub struct NodeStructAttrs {
    pub rename: Option<LitStr>,
    pub has_child: bool,
    pub child_name: Option<LitStr>,
    pub child_index: Option<syn::LitInt>,
    pub get_node_value_with: Option<syn::ExprPath>,
    pub set_node_value_with: Option<syn::ExprPath>,
    pub with_children: Option<syn::ExprPath>,
}

impl NodeStructAttrs {
    /// Parses NodeStructAttrs from the `#[node(...)]` annotation on a struct field.
    /// - `ast_result`: Error context
    /// - `_index`: Field index (reserved for future use)
    /// - `field`: Current field
    ///
    /// If you need multiple child nodes in AI nodes, you can add annotations like
    ///   #[node(child_name="dialogNodes", with_children="my_mod::process_children")]
    /// to `dialog_nodes`, `storage_nodes`, etc., to automatically recognize multiple child nodes during subsequent generation/execution.
    pub fn from_ast(ast_result: &ASTResult, _index: usize, field: &syn::Field) -> Self {
        // Use ASTAttr container to temporarily store 6 key fields
        let mut rename = ASTAttr::none(ast_result, RENAME_NODE);
        let mut child_name = ASTAttr::none(ast_result, CHILD_NODE_NAME);
        let mut child_index = ASTAttr::none(ast_result, CHILD_NODE_INDEX);
        let mut get_node_value_with = ASTAttr::none(ast_result, GET_NODE_VALUE_WITH);
        let mut set_node_value_with = ASTAttr::none(ast_result, SET_NODE_VALUE_WITH);
        let mut with_children = ASTAttr::none(ast_result, WITH_CHILDREN);

        // Collect all meta items under #[node(...)]
        let meta_items = field
            .attrs
            .iter()
            .flat_map(|attr| get_node_meta_items(ast_result, attr))
            .flatten();

        // Iterate over meta items
        for meta_item in meta_items {
            match &meta_item {
                // #[node(key = value)] form
                Meta(NameValue(name_value)) => {
                    parse_node_keyvalue(
                        ast_result,
                        name_value,
                        &mut rename,
                        &mut child_name,
                        &mut child_index,
                        &mut get_node_value_with,
                        &mut set_node_value_with,
                        &mut with_children,
                    );
                },
                // Form like #[node(xxx)] without specifying = value
                Meta(meta_item) => {
                    let path_str = meta_item.path().to_token_stream().to_string();
                    ast_result.error_spanned_by(
                        meta_item.path(),
                        format!("unknown node field attribute `{}`", path_str),
                    );
                },
                // Pure literal => error
                Lit(lit_val) => {
                    ast_result.error_spanned_by(
                        lit_val,
                        "unexpected literal in `#[node(...)]`"
                    );
                }
            }
        }

        // Finally generate NodeStructAttrs
        let child_name_val = child_name.get();
        NodeStructAttrs {
            rename: rename.get(),
            child_index: child_index.get(),
            has_child: child_name_val.is_some(), // If child_name exists, then has_child = true
            child_name: child_name_val,
            get_node_value_with: get_node_value_with.get(),
            set_node_value_with: set_node_value_with.get(),
            with_children: with_children.get(),
        }
    }
}

/// Extracts the "key=value" branches from `#[node(key="value")]` into a separate function to reduce repetition.
/// 
/// - name_value.path => (rename, child_name, child_index, ...)
/// - name_value.lit => specific string/number/etc.
fn parse_node_keyvalue(
    ast_result: &ASTResult,
    name_value: &syn::MetaNameValue,

    rename: &mut ASTAttr<LitStr>,
    child_name: &mut ASTAttr<LitStr>,
    child_index: &mut ASTAttr<syn::LitInt>,
    get_node_value_with: &mut ASTAttr<syn::ExprPath>,
    set_node_value_with: &mut ASTAttr<syn::ExprPath>,
    with_children: &mut ASTAttr<syn::ExprPath>,
) {
    let path = &name_value.path;
    let lit = &name_value.lit;

    // Handle different keys accordingly
    if path == RENAME_NODE {
        // `#[node(rename = "someName")]`
        if let syn::Lit::Str(lit_str) = lit {
            rename.set(path, lit_str.clone());
        }
    } else if path == CHILD_NODE_NAME {
        // `#[node(child_name = "childX")]`
        if let syn::Lit::Str(lit_str) = lit {
            child_name.set(path, lit_str.clone());
        }
    } else if path == CHILD_NODE_INDEX {
        // `#[node(child_index = 123)]`
        if let syn::Lit::Int(lit_int) = lit {
            child_index.set(path, lit_int.clone());
        }
    } else if path == GET_NODE_VALUE_WITH {
        // `#[node(get_node_value_with = "...")]`
        if let Ok(path_expr) = parse_lit_into_expr_path(ast_result, GET_NODE_VALUE_WITH, lit) {
            get_node_value_with.set(path, path_expr);
        }
    } else if path == SET_NODE_VALUE_WITH {
        // `#[node(set_node_value_with = "...")]`
        if let Ok(path_expr) = parse_lit_into_expr_path(ast_result, SET_NODE_VALUE_WITH, lit) {
            set_node_value_with.set(path, path_expr);
        }
    } else if path == WITH_CHILDREN {
        // `#[node(with_children = "...")]`
        if let Ok(path_expr) = parse_lit_into_expr_path(ast_result, WITH_CHILDREN, lit) {
            with_children.set(path, path_expr);
        }
    } else {
        // Unknown key => error
        let path_str = path.to_token_stream().to_string().replace(' ', "");
        ast_result.error_spanned_by(
            path,
            format!("unknown node field attribute `{}`", path_str),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*; // import NodeStructAttrs, parse_node_keyvalue, etc.
    use syn::{parse_quote, Fields, ItemStruct};
    use crate::ctxt::ASTResult; // ASTResult from your `ctxt` mod

    #[test]
    fn test_node_struct_attrs_basic() {
        // 1) 构造一个 struct，字段带 `#[node(...)]` 注解
        let item_struct: ItemStruct = parse_quote! {
            struct Demo {
                #[node(
                    rename = "my_field",
                    child_name = "childNodes",
                    child_index = 2,
                    get_node_value_with = "node_mod::get_val",
                    set_node_value_with = "node_mod::set_val",
                    with_children = "node_mod::handle_children"
                )]
                pub field: Vec<String>,
            }
        };

        // 2) 拿到 `field`
        let field = match &item_struct.fields {
            Fields::Named(named) => &named.named[0],
            _ => panic!("expected named fields"),
        };

        // 3) 解析
        let ast_result = ASTResult::new();
        let node_attrs = NodeStructAttrs::from_ast(&ast_result, 0, field);

        // 4) 检查
        // rename => Some("my_field")
        assert_eq!(node_attrs.rename.as_ref().unwrap().value(), "my_field");
        // child_name => Some("childNodes"), hence has_child=true
        assert!(node_attrs.has_child);
        assert_eq!(node_attrs.child_name.as_ref().unwrap().value(), "childNodes");
        // child_index => Some(2)
        assert_eq!(node_attrs.child_index.as_ref().unwrap().base10_parse::<u32>().unwrap(), 2);

        // get_node_value_with => "node_mod::get_val"
        let get_val = node_attrs.get_node_value_with.as_ref().unwrap().to_token_stream().to_string();
        assert_eq!(get_val, "node_mod :: get_val");

        // set_node_value_with => "node_mod::set_val"
        let set_val = node_attrs.set_node_value_with.as_ref().unwrap().to_token_stream().to_string();
        assert_eq!(set_val, "node_mod :: set_val");

        // with_children => "node_mod::handle_children"
        let wch = node_attrs.with_children.as_ref().unwrap().to_token_stream().to_string();
        assert_eq!(wch, "node_mod :: handle_children");

        // 无错误
        ast_result.check().unwrap();
    }

    #[test]
    fn test_node_struct_attrs_unknown_key() {
        let item_struct: ItemStruct = parse_quote! {
            struct Demo {
                #[node(unknown="???")]
                field: String,
            }
        };
        let field = match &item_struct.fields {
            Fields::Named(named) => &named.named[0],
            _ => panic!("expected named field"),
        };

        let ast_result = ASTResult::new();
        let _node_attrs = NodeStructAttrs::from_ast(&ast_result, 0, field);

        // 检查是否报错: "unknown node field attribute `unknown`"
        match ast_result.check() {
            Ok(()) => panic!("Expected an error for unknown node attribute"),
            Err(errs) => {
                assert!(!errs.is_empty());
                let msg = errs[0].to_string();
                assert!(msg.contains("unknown node field attribute `unknown`"));
            }
        }
    }

    #[test]
    fn test_node_struct_attrs_literal_error() {
        // 这里演示 `[node("some_literal")]` 纯字面量 => 这会被视为 `NestedMeta::Lit`, 代码中要报错
        let item_struct: ItemStruct = parse_quote! {
            struct Demo {
                #[node("some_literal")]
                field: u32,
            }
        };
        let field = match &item_struct.fields {
            Fields::Named(named) => &named.named[0],
            _ => panic!("expected named field"),
        };

        let ast_result = ASTResult::new();
        let _node_attrs = NodeStructAttrs::from_ast(&ast_result, 0, field);

        // 期待报 "unexpected literal in `#[node(...)]`"
        match ast_result.check() {
            Ok(()) => panic!("Expected error for literal in #[node(...)]"),
            Err(errs) => {
                assert!(!errs.is_empty());
                let msg = errs[0].to_string();
                assert!(msg.contains("unexpected literal in `#[node(...)]`"));
            }
        }
    }
}