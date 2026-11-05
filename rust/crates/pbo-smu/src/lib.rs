//! Hardware access through the PawnIO driver.
//!
//! PawnIO is a signed kernel driver that runs small signed modules; this crate loads two of them,
//! `AMDFamily17` (SMN and MSR reads) and `RyzenSMU` (the SMU mailbox and the PM table). The protocol is the
//! driver's own: open `\\?\GLOBALROOT\Device\PawnIO`, load a module with one IOCTL, then call its named
//! functions with another. The modules are embedded unchanged (LGPL-2.1, release 0.2.4 of PawnIO.Modules).
//!
//! Provenance: the protocol is the one used by LibreHardwareMonitor and by the vendored ZenStates-Core,
//! which this was checked against. The register addresses and message IDs are that library's. Every read
//! here is read-only; stage E0 writes nothing to the processor.

#![cfg(windows)]

pub mod amd;
pub mod bus;
pub mod device;
pub mod rsmu;

pub use amd::AmdFamily17;
pub use device::{Error, Module};
pub use rsmu::RyzenSmu;
