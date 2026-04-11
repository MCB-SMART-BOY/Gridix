//! Dialog ownership and orchestration primitives.
//!
//! The UI dialog widgets still live in `ui::dialogs`; this module owns the
//! application-level modal invariant: at most one dialog is allowed to receive
//! keyboard input in a frame.

pub(in crate::app) mod host;

pub(in crate::app) use super::DbManagerApp;
