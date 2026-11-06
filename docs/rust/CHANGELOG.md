# Changelog of the Rust rebuild

Kept apart from the root `CHANGELOG.md`: the C# release workflow uses that whole file as its release text, and nothing of the Rust version has been released.

## 1.2.0-beta.1 — not released

The Rust rebuild (`docs/rust`, `rust/`). One `pbo.exe` is the window and the command line; it takes over what a 1.x
install kept (`pbo import`). Built and tested locally; nothing here has been published.

### Added

- **The window:** the cores with a stepper and a depth bar each, Auto in three levels of thoroughness (the worst-case
  time is shown before the start, and it goes both ways: a core held at a value that failed is searched back up), ready-made
  test lists, and a plan builder (Advanced: what to test, and how to run it) that shows the same thing as a command line.
- **Tests:** Prime95 and y-cruncher on one core, in a job object, with the engine's load measured; load changes (a pause of a
  second every half minute, or sub-second bursts) so that the core has to boost back up; the order of the cores (in turn,
  alternating, random, pairs); rounds; what to do on an error (carry on, leave the core out, stop); a sound at an error and at
  the end; a time of "auto" (until the engine has been through its work list once); an FFT range of one's own; two sets of
  y-cruncher algorithms.
- **After a crash:** the run resumes where it stopped and the crash is held against the value that was running. The report
  names every fatal machine check Windows wrote for that crash, with the core each names. `pbo whea [--days N]` lists the
  machine checks Windows has logged, by crash and by core, with the unexpected shutdowns next to them and a note when the System
  log has been cleared.
- **A helper process with the rights** over a pipe, so that a window without them can drive a run, and an example guest that
  embeds the program's core (`rust/examples/embed`).
- Language-neutral test vectors (`spec/vectors`) that the C# and the Rust program both have to pass.

- **Auto is chosen beforehand, out of sight:** Start is all there is to do; the level of thoroughness, how far each step goes (1 to 5 counts, `--step N` on the command line), how much safety is kept at the end (1 to 5) and where the search of a core that holds nothing begins (-10, -15 or -20, `--start N`). The selection is what the program judges efficient for the processor (quick, step 5, safety 3, start -20 on an X3D and -15 on the others); a button, Fine-tuning, opens them (and goes away; Back, at the top right, closes them again) for whoever wants to change one; the times shown follow them. The BIOS list says how to enter the values in the UEFI and shows sign and magnitude for each core, and a finished run offers it.
- **A window that is a preview writes nothing**: a scripted click in `pbo gui-preview` no longer creates the logon task, saves a profile, forgets the memory, installs a program or writes a value.

### Found while rebuilding

- **Prime95 was started so that it read none of its settings** (a quoted work folder it takes literally, and no `NumCores`), used
  every core of the machine and reported nothing for minutes. The 1.x program starts it that way too and is not changed here.
- **y-cruncher's stress verb cannot say which processors to use**, so it ran 16 threads on the one processor it was confined to.
  It is now started from a settings file that names the logical processors.
- **Zen 4 and Zen 5 read the Curve Optimizer with message 0xD5**, not the 0x7C of Zen 3.
- **The first value of a search depends on the processor:** -20 on an X3D, -15 on the others (1.x: -10 on an X3D, with no reason on record). A core that holds nothing yet is not searched from 0.

### Not verified

- **A real engine error and a real WHEA event.** The detection is tested with the real Prime95 and a line it would write, and
  with an entry in a log of its own; no core of the one machine it was run on (a 5800X3D) has been made to fail.
- **Zen 4 and Zen 5** have not been measured; they are marked as unchecked.
- **A full run over all eight cores**, and a person using the window with a mouse.
- There is **no code signature**; `pbo version` and `pbo hash` give the checksum.

