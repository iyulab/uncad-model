//! Synthetic golden cases for the model and everything built on it.
//!
//! A case is a [`Spec`]: the entities and values of a synthetic drawing, as
//! Rust data. [`write`] turns it into an R2000 ASCII DXF without touching any
//! parser's code, and [`expected`] turns the same spec into the model a
//! parser is expected to produce -- so the spec is the oracle, not a
//! hand-maintained expected file.
//!
//! Test-only, never published: consumers take it as a dev-dependency and run
//! their share of the cases (a parser compares its output to `expected`; a
//! summarizer checks its summary against the spec's values; and so on).

#![forbid(unsafe_code)]

pub mod cases;
pub mod expected;
pub mod spec;
pub mod writer;

pub use spec::{
    AttribSpec, BlockSpec, Codepage, DimStyleSpec, EntitySpec, LayerSpec, LayerState, LayoutSpec,
    Spec, Xy,
};
pub use writer::{write, Handles, Written};
