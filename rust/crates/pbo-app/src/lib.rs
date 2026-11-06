//! What the application knows, without a window and without hardware (ADR-0004).
//!
//! The window reads an [`plan::Plan`] and a [`pbo_core::runstate::State`] and sends intents back; it decides nothing.
//! Every sentence it shows comes from [`text`], where each key has both languages or the program does not build.

pub mod plan;
pub mod planfile;
pub mod text;
pub mod wire;
