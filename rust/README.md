# PboStudio in Rust

The rebuild of PboStudio. Plan, decisions and stage results are in [`docs/rust`](../docs/rust/README.md).
This is stage E0 (the proof): `pbo diag` reads what a machine reports and writes a text report.
It is read-only. It writes nothing to the processor.

```
cargo test --workspace
cargo build --release
```

Run `target\release\pbo.exe diag` as administrator. It writes `pbo-diag.txt` next to the program.
`--raw` adds the whole power monitoring table, which is what helps most on a Zen 4 or Zen 5 part.

| Crate | What it is |
|---|---|
| `pbo-core` | pure logic, no dependencies at all |
| `pbo-os` | threads, CPUID, child processes in a job object |
| `pbo-smu` | the PawnIO driver, SMN and MSR reads, the SMU mailbox, the PM table |
| `pbo` | the program |

The PawnIO modules in `external/ZenStates-Core/Resources/PawnIo` are signed and used unchanged (LGPL-2.1).
Licence of the Rust code: GPL-3.0, like the rest of the repository.
