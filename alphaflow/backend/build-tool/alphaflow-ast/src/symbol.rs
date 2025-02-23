use std::fmt::{self, Display};
use syn::{Ident, Path};

/// `Symbol` is a lightweight wrapper around &'static str,
/// used in compile-time procedural macro/annotation parsing to manage various keywords,
/// and simplifies comparison logic with `PartialEq<Symbol>` (e.g., `if path == PB_INDEX { ... }`).
#[derive(Copy, Clone, Debug)]
pub struct Symbol(&'static str);

/// ----------------------------------------------------------------------------
/// Protobuf annotation (#[pb(...)]) related Symbol constants
/// ----------------------------------------------------------------------------

/// Top-level annotation: `#[pb(...)]`
pub const PB_ATTRS: Symbol = Symbol("pb");

/// On fields: `#[pb(skip)]`
pub const SKIP: Symbol = Symbol("skip");

/// On fields: `#[pb(index = "...")]`
pub const PB_INDEX: Symbol = Symbol("index");

/// On fields: `#[pb(one_of)]`
pub const PB_ONE_OF: Symbol = Symbol("one_of");

/// On fields: `#[pb(skip_pb_deserializing)]`
pub const SKIP_PB_DESERIALIZING: Symbol = Symbol("skip_pb_deserializing");

/// On fields: `#[pb(skip_pb_serializing)]`
pub const SKIP_PB_SERIALIZING: Symbol = Symbol("skip_pb_serializing");

/// On fields: `#[pb(serialize_pb_with = "...")]`
pub const SERIALIZE_PB_WITH: Symbol = Symbol("serialize_pb_with");

/// On fields: `#[pb(deserialize_pb_with = "...")]`
pub const DESERIALIZE_PB_WITH: Symbol = Symbol("deserialize_pb_with");

/// On types: `#[pb(struct="some struct")]`
pub const PB_STRUCT: Symbol = Symbol("struct");

/// On types: `#[pb(enum="some enum")]`
pub const PB_ENUM: Symbol = Symbol("enum");

/// ----------------------------------------------------------------------------
/// Event annotation (#[event(...)]) related Symbol constants
/// ----------------------------------------------------------------------------

/// Top-level annotation: `#[event(...)]`
pub const EVENT: Symbol = Symbol("event");

/// On fields: `#[event(input="...")]`
pub const EVENT_INPUT: Symbol = Symbol("input");

/// On fields: `#[event(output="...")]`
pub const EVENT_OUTPUT: Symbol = Symbol("output");

/// On fields: `#[event(ignore)]`
pub const EVENT_IGNORE: Symbol = Symbol("ignore");

/// On enums: `#[event_err="..."]`
pub const EVENT_ERR: Symbol = Symbol("event_err");

/// ----------------------------------------------------------------------------
/// Node annotation (#[node(...)]) related Symbol constants
/// ----------------------------------------------------------------------------

/// Top-level annotation: `#[node(...)]`
pub const NODE_ATTRS: Symbol = Symbol("node");

/// Top-level annotation (plural): `#[nodes(...)]`
pub const NODES_ATTRS: Symbol = Symbol("nodes");

/// `#[node_type = "..."]`
pub const NODE_TYPE: Symbol = Symbol("node_type");

/// `#[node(index="...")]`
pub const NODE_INDEX: Symbol = Symbol("index");

/// `#[node(rename="someName")]`
pub const RENAME_NODE: Symbol = Symbol("rename");

/// `#[node(child_name="childNodes")]`
pub const CHILD_NODE_NAME: Symbol = Symbol("child_name");

/// `#[node(child_index=123)]`
pub const CHILD_NODE_INDEX: Symbol = Symbol("child_index");

/// `#[node(skip_node_attribute)]`
pub const SKIP_NODE_ATTRS: Symbol = Symbol("skip_node_attribute");

/// `#[node(get_value_with="...")]`
pub const GET_NODE_VALUE_WITH: Symbol = Symbol("get_value_with");

/// `#[node(set_value_with="...")]`
pub const SET_NODE_VALUE_WITH: Symbol = Symbol("set_value_with");

/// `#[node(get_element_with="...")]`
pub const GET_VEC_ELEMENT_WITH: Symbol = Symbol("get_element_with");

/// `#[node(get_mut_element_with="...")]`
pub const GET_MUT_VEC_ELEMENT_WITH: Symbol = Symbol("get_mut_element_with");

/// `#[node(with_children="...")]`
pub const WITH_CHILDREN: Symbol = Symbol("with_children");

/// ----------------------------------------------------------------------------
/// Implement PartialEq<Symbol> for Ident / Path
/// ----------------------------------------------------------------------------

impl PartialEq<Symbol> for Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

/// ----------------------------------------------------------------------------
/// Implement Display for Symbol
/// ----------------------------------------------------------------------------

impl Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    use syn::{Ident, Path};

    #[test]
    fn test_symbol_ident_eq() {
        // parse_quote! 强制解析一个标识符 "index"
        let ident: Ident = parse_quote!(index);
        // 断言 ident == PB_INDEX
        assert_eq!(ident, PB_INDEX);
        assert!(ident == PB_INDEX, "Should match PB_INDEX");

        // parse_quote!(foo) => "foo" ident
        let foo: Ident = parse_quote!(foo);
        assert_ne!(foo, PB_INDEX, "foo != PB_INDEX");
    }

    #[test]
    fn test_symbol_path_eq() {
        // parse a Path "pb"
        let path: Path = parse_quote!(pb);
        // 断言 path == PB_ATTRS
        assert_eq!(path, PB_ATTRS);

        let skip_path: Path = parse_quote!(skip);
        assert_eq!(skip_path, SKIP);

        // parse_quote!(event)
        let event_path: Path = parse_quote!(event);
        // 断言 event_path == EVENT
        assert_eq!(event_path, EVENT);

        // parse something that won't match
        let random_path: Path = parse_quote!(randomstuff);
        assert_ne!(random_path, PB_ATTRS);
        assert_ne!(random_path, EVENT);
    }

    #[test]
    fn test_symbol_display() {
        let s = format!("{}", PB_INDEX);
        assert_eq!(s, "index");

        let s2 = format!("{}", NODE_ATTRS);
        assert_eq!(s2, "node");
    }
}