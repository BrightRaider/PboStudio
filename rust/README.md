# PboStudio in Rust

The rebuild of PboStudio. Plan, decisions and stage results are in [`docs/rust`](../docs/rust/README.md).
`pbo.exe` is one file with two faces: started without arguments it opens the window, with a command it is a command line
program. Both run the same code.

```
cargo test --workspace
cargo build --release
```

## Commands

Run as administrator: the processor registers are only reachable that way.

| Command | What it does |
|---|---|
| `pbo` | the window (it asks for administrator rights once at start and carries on as that copy; declined, it opens without them and offers a helper; `PBO_NO_ELEVATE` skips the question): the cores with a stepper and a depth bar each, Auto with three levels of thoroughness (the worst-case time is shown before the start), ready-made test lists, and a plan builder |
| `pbo diag [--raw]` | reads what this machine reports and writes `pbo-diag.txt` next to the program. Read-only. `--raw` adds the whole power monitoring table, which helps most on a Zen 4 or Zen 5 part. |
| `pbo status` | the Curve Optimizer value of every core and the live readings ; `on --continue` makes the task also carry Auto on after a crash |
| `pbo selftest [--step]` | writes each core's own value back and reads it (changes nothing); `--step` also moves each core one step toward zero and puts it back |
| `pbo run` | tests the cores at the values they hold; changes nothing on the processor |
| `pbo tune` | searches each core's value, always through the checked write path; puts values back if cancelled |
| `pbo auto [--intensity fast|normal|thorough]` | the window's Auto without a window: the three phases one after the other (what to run comes from the same campaign), until it is done, has nothing new to find, or needs the driver or a test program. It writes down how it was started, so that `pbo resume --continue` can carry it on after a crash |
| `pbo set CORE VALUE` | one value, written and read back |
| `pbo plan new\|show\|command\|run FILE` | the plan as a readable file (a small subset of TOML) that the window, the command line and the helper all use |
| `pbo helper [--pipe NAME]`, `pbo helper-call NAME MESSAGE` | the part with the rights, served over a pipe to a window or module without them; one message, one answer |
| `pbo hash FILE...`, `pbo verify FILE SHA256`, `pbo version` | SHA-256 of files and of this program; there is no code signature, so the checksum is how a release is told from a copy |
| `pbo setup status\|prime95\|ycruncher\|all` | puts the test programs into `engines` next to the program: https from the vendor's own domain only, and the archive must have the SHA-256 this version was made with (`--from ARCHIVE` installs offline, `--accept-unverified` is your explicit choice) |
| `pbo update [--beta]` | says whether a newer release exists (stable, or beta too); it only looks, and never replaces the program ; the releases of this program are the ones tagged `rust-v...`, the C# program has its own |
| `pbo setup pawnio` | starts the PawnIO driver installer (Windows asks for permission) after the same checks as the test programs: SHA-256 of this version's installer, a fresh private folder for a download, nothing started that does not match |
| `pbo estimate [--cores ..] [--intensity fast\|normal\|thorough]` | how long the campaign still has to run at that thoroughness (balanced by default), phase by phase, calculated from what each core holds and may reach |
| `pbo ready [--hours N]` | "Ready for the night?": the logon task, automatic sign-in, the sleep timer, free disk space, temperature headroom; changes nothing, says what is missing |
| `pbo save`, `pbo apply` | keep the current values as the logon profile; write that profile |
| `pbo resume` | after a crash: put every core back to what it was proven to hold ; `--continue` also carries `pbo auto` on after a countdown (`--wait N`, a minute by default, stopped by `pbo resume --cancel`), at most three times in a row |
| `pbo autostart on\|off\|status` | the task that runs `pbo resume` at logon (never at system start) |
| `pbo knowledge` | what the program remembers about each core |
| `pbo report` | one file, `pbo-report.txt`, to attach to an issue: version, what the processor holds, what the program remembers, the machine checks of 90 days, how Auto was started, the end of the newest log, and `pbo-diag.txt` if there is one |
| `pbo whea [--days N]` | the machine checks Windows has logged (a year by default): one line for each crash with the cores it names (and the unexpected shutdown, Kernel-Power 41, logged just before it), a count of the shutdowns that had no machine check next to them, a count for each core, and a note when the System log has been cleared (what it held before is gone). Reads only, needs no rights |
| `pbo import [FOLDER]` | takes over what a 1.x install kept (memory, profile, settings, logs); overwrites nothing |
| `pbo gui-preview NAME FILE.png [light] [de] [size=WxH] [click=X,Y ...]` | draws one window state with made-up numbers and no hardware; the clicks go through the window's own input |

Options of `run`, `tune` and `resume`: `--seconds N`, `--cores 0,1`, `--engine prime95|ycruncher`, `--fft NAME`, `--smt`,
`--fine`, `--step N` (steps of N counts, 1 to 6), `--start N` (where the search of a core that holds nothing begins, 1 to 30 counts below zero), `--pause N` (seconds of rest between two cores), `--yc-tests A,B` and `--yc-memory MB` (y-cruncher's tests, by their tags, and its memory for each logical processor), `--guardband N` (the safety kept at the end), `--mode sse|avx|avx2|avx512`, `--then ENGINE:MODE:FFT+...` (further steps per core),
`--rounds N` (how often a test of the held values goes round all cores), `--order sequential|alternate|random|pairs`
(the order of the cores), `--on-error continue|skip|stop` (what a test of the held values does when a core fails; a failed core is left out of the later rounds by default), `--quiet` (no sound at an error and at the end), `--yc-set curve-optimizer|fast-discovery`, `--seconds auto` (until the test program has been through its work list once, an hour at most) and `--fft MIN-MAX` (a range of one's own, in K) and `--transient off|bursts|periodic[:EVERY:PAUSE]` (how the load is interrupted so that the core has
to boost back up: a pause of a second every half minute by default, or sub-second bursts). The test programs are looked for in an `engines` folder next to the program
(`engines\prime95\prime95.exe`, `engines\ycruncher\...`) or in the folder named by `PBO_ENGINES`. Only Windows writes real machine checks (WHEA events); to try the whole way an event takes
without one, `PBO_TEST_EVENTLOG=Log|Provider` names a second event log and provider to read from, in addition to the real
source and never instead of it (create the log with `New-EventLog`, write an event with id 19 into it while a core is under test).

Files are kept in `runs\` next to the program, in the formats 1.x writes: `core_knowledge.json`, `co_saved.json`,
`settings.json` and `logs\`. There is one log per day (`pbo_YYYY-MM-DD.log`); the log day ends at 06:00, so a night
belongs to the evening it began, and nothing is deleted automatically.

## Crates

| Crate | What it is |
|---|---|
| `pbo-core` | pure logic, no dependencies at all |
| `pbo-app` | the plan, the words (German and English), what the window may show; no window, no hardware |
| `pbo-ui` | the look for egui: colours, gradients, glass-edged cards, ring gauges, steppers, pills, tiles, glowing buttons |
| `pbo-os` | threads, CPUID, child processes in a job object, clock, console |
| `pbo-smu` | the PawnIO driver, SMN and MSR, the SMU mailboxes, the PM table |
| `pbo` | the program |

The PawnIO modules in `external/ZenStates-Core/Resources/PawnIo` are signed and used unchanged (LGPL-2.1).
Licence of the Rust code: GPL-3.0, like the rest of the repository.

## Sicherheit und Verträglichkeit (safety and compatibility)

This program changes the voltage curve of a processor and uses a kernel driver. It is said once, here, and the decision is
yours.

- **The driver.** Hardware access goes through PawnIO, a signed, general-purpose driver (LGPL-2.1) with small signed
  modules. It is not loaded at system start but on demand, and once loaded it stays loaded until the next restart.
  The modules are used unchanged.
- **Rights.** Administrator rights are needed because the processor registers are reachable no other way. `pbo diag`,
  `status` and the previews write nothing; only `selftest`, `tune`, `set`, `apply` and `resume` write, and each write is
  read back.
- **Values are live.** Curve Optimizer values are written to the running processor and are discarded at the next
  restart. Nothing is written to the BIOS. A value that does not hold can crash the machine; that is what the program is
  for, and it is why a cut-off run is recognised after the restart, the proven values are put back, and a value that
  crashed is never offered again.
- **What is checked on hardware.** Only Zen 3 (a Ryzen 7 5800X3D, Vermeer): reading and writing values, the power
  monitoring table, the core voltage. Zen 4 and Zen 5 are *not* checked: the program says so in the window and in
  `pbo diag`, and on such a processor the write path tests itself first (each core's own value is written back and read)
  before anything else is written. Reports from those processors are what closes this gap.
- **Not investigated: anti-cheat.** There is one observation and no study. On the author's gaming machine the driver was
  loaded while games with Riot Vanguard and Easy Anti-Cheat installed ran without complaint. That is one machine; it says
  nothing about other anti-cheat systems, versions or machines. PboStudio does not promise compatibility. Whoever starts it
  decides that.
- **Not checked yet.** Machine check (WHEA) detection has not been seen against a real event on the checked machine, the
  logon task has not been created on a real machine, and `pbo setup` has been run against local archives but not against the
  vendors' servers.
- **Windows' own tools.** The few it starts (`schtasks`, `wevtutil`, `powercfg`, `curl`, `tar`) are started by their full path in
  the system folder, never by bare name, and with a limit on the wait. A program that runs as administrator would otherwise start
  whatever sits next to it under the right name, and a tool that hangs would freeze the test that asked. Downloads (`pbo setup`)
  are https from the vendor's own domain only, and an archive is unpacked only when its SHA-256 is the one this version was made
  with.

## Release

```
cargo build --release
```

gives `target\release\pbo.exe`, one file. Publish it with its checksum (`pbo hash pbo.exe` writes the line `sha256sum` checks) and say in the release notes
that it is not code-signed, so the checksum is how a copy is told from the release. The version is in `Cargo.toml` (`1.2.0` is the first release); `pbo version` prints it with the checksum of the running file. The tag of a release of this program starts with `rust-` (`rust-v1.2.0`); one with `-beta` in it is a pre-release, which is what `pbo update --beta` looks for.
