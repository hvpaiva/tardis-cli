//! AST resolver: maps DateExpr nodes to concrete jiff::Zoned datetimes.
//!
//! This module is a pure function of AST + reference time. No parsing logic.
//! All datetime arithmetic uses jiff's native calendar-aware operations.
//! Clamping policy (PARS-07): jiff's checked_add/checked_sub clamps to
//! end-of-month (e.g., Jan 31 + 1 month = Feb 28). This is intentional
//! and matches Python dateutil, JS Temporal, and Go time.AddDate behavior.

// Stub: full implementation in Task 2.
#![allow(dead_code)]

use jiff::Zoned;

use crate::parser::{ast::*, error::ParseError};

/// Resolve an AST node to a concrete `jiff::Zoned` datetime.
pub(crate) fn resolve(expr: &DateExpr, now: &Zoned) -> Result<Zoned, ParseError> {
    let _ = (expr, now);
    Err(ParseError::unsupported(
        "resolver not yet implemented (stub for grammar compilation)",
    ))
}
