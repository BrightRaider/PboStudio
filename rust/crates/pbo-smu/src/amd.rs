//! The `AMDFamily17` module: reads of the system management network (SMN) and of model-specific registers.

use crate::bus::PciBusLock;
use crate::device::{Error, Module};

static MODULE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../external/ZenStates-Core/Resources/PawnIo/AMDFamily17.bin"
));

pub struct AmdFamily17 {
    module: Module,
}

impl AmdFamily17 {
    pub fn open() -> Result<AmdFamily17, Error> {
        Ok(AmdFamily17 {
            module: Module::load(MODULE)?,
        })
    }

    /// One SMN register, under the PCI bus lock. `None` when the read fails.
    pub fn read_smn(&self, address: u32) -> Option<u32> {
        let _lock = PciBusLock::acquire(5000)?;
        let out = self
            .module
            .execute("ioctl_read_smn", &[i64::from(address)], 1)
            .ok()?;
        out.first().map(|v| *v as u32)
    }

    /// One MSR on whichever processor this thread currently runs on.
    pub fn read_msr(&self, index: u32) -> Option<u64> {
        let out = self
            .module
            .execute("ioctl_read_msr", &[i64::from(index)], 1)
            .ok()?;
        out.first().map(|v| *v as u64)
    }

    /// One MSR on a chosen logical processor: the thread is pinned for the call and put back afterwards.
    pub fn read_msr_on(&self, logical: usize, index: u32) -> Option<u64> {
        let _pin = pbo_os::pin_current_thread(logical)?;
        self.read_msr(index)
    }
}
