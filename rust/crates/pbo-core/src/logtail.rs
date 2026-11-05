//! Reading the new, complete lines of a log file that another process is still writing (invariant I2).
//!
//! Both stress engines report a failed calculation by writing a line into a file that is polled once a second.
//! Reading up to the end of the file and resuming from its length missed a failure whenever the engine was part
//! way through a line: "FATAL ERROR" cut across two polls matched neither half, and the core passed. A missed
//! failure is the one mistake a stability test cannot afford, so nothing past the last newline is consumed until
//! it has a newline after it. This is the pure half: it works on the file's bytes; reading the file is the
//! operating system layer's job. Vectors: `spec/vectors/log-tail.json`.

/// Where the previous poll stopped. Advanced only over lines that were returned.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LogTail {
    pub offset: usize,
}

impl LogTail {
    /// The new complete lines. `content` is the whole file as it is now, or `None` when it cannot be read (it
    /// is missing, or the engine holds it for a moment): that yields nothing and moves nothing, and the next
    /// poll picks it up. A file that has shrunk since is read again from the start.
    ///
    /// Empty lines with a bare newline are dropped; a line that is only a carriage return comes back as an
    /// empty string, as it always did.
    pub fn poll(&mut self, content: Option<&[u8]>) -> Vec<String> {
        let Some(content) = content else {
            return Vec::new();
        };

        if content.len() < self.offset {
            self.offset = 0;
        }
        if content.len() == self.offset {
            return Vec::new();
        }

        let fresh = &content[self.offset..];
        let Some(last_newline) = fresh.iter().rposition(|b| *b == b'\n') else {
            // A line is still being written.
            return Vec::new();
        };

        self.offset += last_newline + 1;

        String::from_utf8_lossy(&fresh[..=last_newline])
            .split('\n')
            .filter(|piece| !piece.is_empty())
            .map(|piece| piece.trim_end_matches('\r').to_string())
            .collect()
    }
}
