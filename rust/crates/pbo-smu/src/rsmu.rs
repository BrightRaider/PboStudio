//! The `RyzenSMU` module: the SMU mailbox and the power monitoring (PM) table.

use crate::bus::PciBusLock;
use crate::device::{Error, Module};

static MODULE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../external/ZenStates-Core/Resources/PawnIo/RyzenSMU.bin"
));

pub struct RyzenSmu {
    module: Module,
}

impl RyzenSmu {
    pub fn open() -> Result<RyzenSmu, Error> {
        Ok(RyzenSmu {
            module: Module::load(MODULE)?,
        })
    }

    /// The SMU firmware version.
    pub fn version(&self) -> Option<u32> {
        let _lock = PciBusLock::acquire(5000)?;
        let out = self.module.execute("ioctl_get_smu_version", &[], 1).ok()?;
        out.first().map(|v| (*v & 0xFFFF_FFFF) as u32)
    }

    /// Sends a message to the RSMU mailbox with six arguments and returns the six answer words.
    /// The module supports this one mailbox only.
    pub fn send_command(&self, message: u32, args: [u32; 6]) -> Result<[u32; 6], Error> {
        let _lock = PciBusLock::acquire(5000).ok_or(Error::CallFailed(-1))?;
        let mut input = [0i64; 7];
        input[0] = i64::from(message);
        for (i, a) in args.iter().enumerate() {
            input[i + 1] = i64::from(*a);
        }
        let out = self.module.execute("ioctl_send_smu_command", &input, 6)?;
        let mut answer = [0u32; 6];
        for (i, word) in out.iter().take(6).enumerate() {
            answer[i] = *word as u32;
        }
        Ok(answer)
    }

    /// The PM table version and the DRAM address it is published at.
    pub fn resolve_pm_table(&self) -> Option<(u32, u64)> {
        let _lock = PciBusLock::acquire(5000)?;
        let out = self
            .module
            .execute("ioctl_resolve_pm_table", &[0, 0], 2)
            .ok()?;
        match out.as_slice() {
            [version, base, ..] => Some(((*version & 0xFFFF_FFFF) as u32, *base as u64)),
            _ => None,
        }
    }

    /// Asks the SMU to refresh the table, then reads `size_bytes` of it as floats.
    pub fn read_pm_table(&self, size_bytes: usize) -> Option<Vec<f32>> {
        {
            let _lock = PciBusLock::acquire(5000)?;
            self.module.execute("ioctl_update_pm_table", &[], 0).ok()?;
        }
        let words = size_bytes.div_ceil(8);
        let out = self
            .module
            .execute("ioctl_read_pm_table", &vec![0i64; words], words)
            .ok()?;
        let mut bytes = Vec::with_capacity(out.len() * 8);
        for word in &out {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.truncate(size_bytes);
        let (floats, _rest) = bytes.as_chunks::<4>();
        Some(floats.iter().map(|c| f32::from_le_bytes(*c)).collect())
    }
}
