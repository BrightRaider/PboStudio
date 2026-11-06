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

/// One of the processor's message mailboxes: three SMU registers. Zen 3 and Zen 4 take the Curve Optimizer set
/// message through MP1; the module's own `ioctl_send_smu_command` reaches RSMU only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mailbox {
    pub message: u32,
    pub response: u32,
    pub argument: u32,
}

/// The MP1 mailbox of Zen 3 and Zen 4 (the vendored library's addresses, the same for both).
pub const MP1: Mailbox = Mailbox {
    message: 0x03B1_0530,
    response: 0x03B1_057C,
    argument: 0x03B1_09C4,
};

/// The SMU message that sets one core's Curve Optimizer margin through MP1 (Zen 3 and Zen 4).
pub const MP1_SET_CORE_MARGIN: u32 = 0x35;

const RESPONSE_OK: u32 = 0x01;
/// How many times the response register is read before giving up, as in the vendored library.
const WAIT_READS: u32 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailboxError {
    /// The PCI bus lock could not be taken in time.
    Bus,
    /// A register read or write through the driver failed.
    Register,
    /// The mailbox was still busy with an earlier message, or never answered this one.
    Timeout,
    /// The SMU answered with a status other than OK (0xFE unknown message, 0xFD prerequisite, 0xFC busy, 0xFF failed).
    Status(u32),
}

impl std::fmt::Display for MailboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailboxError::Bus => f.write_str("the PCI bus lock could not be taken"),
            MailboxError::Register => f.write_str("a register access failed"),
            MailboxError::Timeout => f.write_str("the mailbox did not answer in time"),
            MailboxError::Status(0xFE) => f.write_str("the SMU does not know this message"),
            MailboxError::Status(0xFD) => {
                f.write_str("the SMU rejected the message (prerequisite)")
            }
            MailboxError::Status(0xFC) => f.write_str("the SMU is busy"),
            MailboxError::Status(s) => write!(f, "the SMU answered with status 0x{s:02X}"),
        }
    }
}

impl std::error::Error for MailboxError {}

impl RyzenSmu {
    fn read_register(&self, address: u32) -> Option<u32> {
        let out = self
            .module
            .execute("ioctl_read_smu_register", &[i64::from(address)], 1)
            .ok()?;
        out.first().map(|v| *v as u32)
    }

    fn write_register(&self, address: u32, value: u32) -> bool {
        self.module
            .execute(
                "ioctl_write_smu_register",
                &[i64::from(address), i64::from(value)],
                0,
            )
            .is_ok()
    }

    fn wait_done(&self, mailbox: Mailbox) -> bool {
        for _ in 0..WAIT_READS {
            if self.read_register(mailbox.response).is_some_and(|v| v != 0) {
                return true;
            }
        }
        false
    }

    /// Sends one message through `mailbox`, the way the vendored library does: wait until the mailbox is idle, clear
    /// the response, write the arguments, write the message, wait for the answer, read the status and the answer
    /// words. The whole exchange runs under the PCI bus lock.
    pub fn send_mailbox(
        &self,
        mailbox: Mailbox,
        message: u32,
        args: [u32; 6],
    ) -> Result<[u32; 6], MailboxError> {
        let _lock = PciBusLock::acquire(5000).ok_or(MailboxError::Bus)?;
        if !self.wait_done(mailbox) {
            return Err(MailboxError::Timeout);
        }
        if !self.write_register(mailbox.response, 0) {
            return Err(MailboxError::Register);
        }
        for (i, a) in args.iter().enumerate() {
            if !self.write_register(mailbox.argument + 4 * i as u32, *a) {
                return Err(MailboxError::Register);
            }
        }
        if !self.write_register(mailbox.message, message) {
            return Err(MailboxError::Register);
        }
        if !self.wait_done(mailbox) {
            return Err(MailboxError::Timeout);
        }
        let status = self
            .read_register(mailbox.response)
            .ok_or(MailboxError::Register)?;
        if status != RESPONSE_OK {
            return Err(MailboxError::Status(status));
        }
        let mut answer = [0u32; 6];
        for (i, slot) in answer.iter_mut().enumerate() {
            *slot = self
                .read_register(mailbox.argument + 4 * i as u32)
                .ok_or(MailboxError::Register)?;
        }
        Ok(answer)
    }

    /// Sets one core's Curve Optimizer margin. `core_mask` comes from `pbo_core::margin::core_mask`. The caller reads
    /// the value back (message 0x7C through [`RyzenSmu::send_command`]) to see whether it stuck.
    pub fn set_core_margin(&self, core_mask: u32, margin: i32) -> Result<(), MailboxError> {
        let argument = pbo_core::margin::set_argument(core_mask, margin);
        self.send_mailbox(MP1, MP1_SET_CORE_MARGIN, [argument, 0, 0, 0, 0, 0])
            .map(|_| ())
    }
}
