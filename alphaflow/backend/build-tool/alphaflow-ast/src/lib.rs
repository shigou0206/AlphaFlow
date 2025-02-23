#![allow(clippy::all)] 

// Remove #[macro_use] if not using macros from syn
extern crate syn;

// Internal modules
mod ast;
mod ctxt;
mod pb_attrs;
mod event_attrs;
mod node_attrs;

pub mod symbol;
pub mod ty_ext;

// Re-exports: make them accessible to external users
pub use ast::*;
pub use ctxt::ASTResult;
pub use pb_attrs::*;
pub use event_attrs::{EventAttrs, EventEnumAttrs, get_event_meta_items};

// symbol / ty_ext often used across the crate, so we re-export them fully:
pub use symbol::*;
pub use ty_ext::*;