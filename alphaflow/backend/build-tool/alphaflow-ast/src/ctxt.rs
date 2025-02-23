use quote::ToTokens;
use std::{
    cell::{Cell, RefCell},
    fmt::Display,
    mem,
    thread,
};

/// `ASTResult` collects `syn::Error` objects during AST parsing/transformation.
/// - Errors can be pushed with `push_error(...)` or `push_spanned(...)`.
/// - Call `check()` at the end to either get Ok(()) if no errors, or Err(Vec<syn::Error>) if any.
/// - If `check()` is not called before this struct is dropped, a warning is printed but no panic occurs.
#[derive(Default)]
pub struct ASTResult {
    errors: RefCell<Vec<syn::Error>>,
    is_checked: Cell<bool>,
}

impl ASTResult {
    /// Create a new empty `ASTResult`.
    pub fn new() -> Self {
        Self {
            errors: RefCell::new(Vec::new()),
            is_checked: Cell::new(false),
        }
    }

    /// Push an existing `syn::Error`.
    pub fn push_error(&self, err: syn::Error) {
        self.errors.borrow_mut().push(err);
    }

    /// Create a `syn::Error` by spanning `obj` with message `msg`, then push it.
    pub fn push_spanned<A: ToTokens, M: Display>(&self, obj: A, msg: M) {
        let tokens = obj.into_token_stream();
        let err = syn::Error::new_spanned(tokens, msg);
        self.push_error(err);
    }

    /// Compatibility method: same as `push_spanned(...)`.
    pub fn error_spanned_by<A: ToTokens, M: Display>(&self, obj: A, msg: M) {
        self.push_spanned(obj, msg);
    }

    /// Compatibility method: same as `push_error(...)`.
    pub fn syn_error(&self, err: syn::Error) {
        self.push_error(err);
    }

    /// Consume `self` and return Ok(()) if no errors, or Err(errors) if there were any.
    /// 
    /// Uses `std::mem::take` to safely extract the error list without partially moving out of `self`.
    pub fn check(self) -> Result<(), Vec<syn::Error>> {
        self.is_checked.set(true);

        // Borrow the RefCell, then mem::take() the Vec to avoid E0509 partial move error.
        let mut inner = self.errors.borrow_mut();
        let errs = mem::take(&mut *inner);

        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

impl Drop for ASTResult {
    fn drop(&mut self) {
        // If user never called check() and we do have errors, print a warning.
        // No panic here, just eprintln. If you prefer forcing an error, you can panic! here.
        if !thread::panicking() && !self.is_checked.get() {
            let count = self.errors.borrow().len();
            if count > 0 {
                eprintln!(
                    "Warning: ASTResult dropped without check(), {} error(s) were not handled!",
                    count
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use syn::Error;

    #[test]
    fn test_no_error() {
        let result = ASTResult::new();
        // No error added => check() => Ok(())
        let outcome = result.check();
        assert!(outcome.is_ok(), "Expected Ok, got Err");
    }

    #[test]
    fn test_push_error() {
        let result = ASTResult::new();
        // Manually create a syn::Error
        let err = Error::new(Span::call_site(), "some error");
        result.push_error(err);

        // check => should return that error
        match result.check() {
            Ok(_) => panic!("Expected error, got Ok"),
            Err(errs) => {
                assert_eq!(errs.len(), 1);
                assert!(errs[0].to_string().contains("some error"));
            }
        }
    }

    #[test]
    fn test_push_spanned() {
        let result = ASTResult::new();
        result.push_spanned("my_tokens", "my message");
        let outcome = result.check();
        match outcome {
            Ok(_) => panic!("Should have had an error"),
            Err(errs) => {
                assert_eq!(errs.len(), 1);
                assert!(errs[0].to_string().contains("my message"));
            }
        }
    }

    #[test]
    fn test_error_spanned_by() {
        let result = ASTResult::new();
        result.error_spanned_by("some tokens", "my error");
        let outcome = result.check();
        assert!(outcome.is_err());
        let errs = outcome.err().unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].to_string().contains("my error"));
    }

    #[test]
    fn test_syn_error() {
        let result = ASTResult::new();
        let err = Error::new(Span::call_site(), "test syn_error");
        result.syn_error(err);
        let errs = result.check().unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].to_string().contains("test syn_error"));
    }
}