# Changelog of the Rust rebuild

Kept apart from the root `CHANGELOG.md`: the C# release workflow uses that whole file as its release text, and nothing of the Rust version has been released.

## 1.2.0 — 2026-11-06

The first release of the Rust version. It contains everything of 1.2.0-beta.1 (below) and what has been added since.

### Added

- **Auto without a window and after a crash:** `pbo auto` runs the campaign of the window's Auto on the command line. After a crash, `pbo resume --continue` (the logon task made with `pbo autostart on --continue`, or the window's Fine-tuning, *After a crash: Carry on*) puts the proven values back and carries Auto on after a countdown of a minute that `pbo resume --cancel` stops, at most three times in a row. Off by default. The **PboStudio window** opens at once (in the same program; no console stays), counts the minute down in the window, where *Do not carry on* stops it, and then runs Auto in view, step after step to the end of the campaign, until *Stop* is pressed; without a desktop it carries on in the console as `pbo auto`. A click into that console can no longer freeze the countdown (quick edit mode is switched off for `resume` and `auto`).
- **Load change "game"** (`--transient game`; the window: Advanced, Run, Load changes, Game): the rhythm of a game such as League of Legends or Teamfight Tactics, for crashes in play that a steady test does not find: spikes of 2 to 8 ms of load about a hundred times a second, irregular, and quiet stretches with a single spike every 40 to 250 ms. With a light load (Prime95 SSE, smallest FFTs) the core stays at peak boost and keeps jumping to it out of idle. The check that the test program used its core now measures against the share of the time the load really ran (for bursts too). Seen on a real process; not yet against a core that failed with it.
- **Smaller things:** `--pause N` between two cores, y-cruncher's tests of one's own (`--yc-tests`) and its memory (`--yc-memory`), and `pbo report`, one file to attach to an issue.
- **`pbo update` looks at the releases of this program only** (tags `rust-v...`); the C# program's own releases are not offered.
- **Known: Windows Defender may flag the program** (`Behavior:Win32/Execution.A!ml`, a behaviour detection of the unsigned program that needs administrator rights, a driver and a logon task). During development it ended the logon task's run, removed the task and quarantined the file once. See the guide for what to do.
- **Auto was run through all three phases on three cores of a 5800X3D** (about an hour, no error, no machine check, the time shown beforehand was the time it took), see `docs/rust/evidence`.
- **The way back after a crash was seen on the 5800X3D**: a forced restart (`shutdown /r /f`) in the middle of Auto, and at logon the crash was noticed and held against core 4, the value backed off, and after the countdown of a minute Auto went on with the arguments it had been started with. A first attempt of this test found a real bug (the file that says how Auto was started was not written on a machine where the folder `runs` did not exist yet); it is fixed and tested. Evidence: `docs/rust/evidence/e6-5800x3d-forced-restart-and-way-back.txt`. This was seen with the logon task after a real forced restart (the window opened without a console, counted down and started Auto by itself); that the window takes the next step by itself and ends the campaign, the countdown in the window and stopping it were seen without a reboot.

### Fixed in the review before the release

The first three were each seen on the 5800X3D: the console closed while core 4 was under test at -30 (it was put back to -28 before the program ended), `pbo auto` closed during its first test (stopped, nothing left behind), and `pbo resume` after a crash with no test programs next to it (core 4 put back to -24).

- **Putting the proven values back after a crash no longer needs a test program:** `pbo resume` (and the window's button) failed when Prime95 was not where it had been, and then wrote nothing back.
- **`pbo auto` stops cleanly on Ctrl+C**, and **closing the console in the middle of a test** waits the second or two it takes to stop the test program and put the value back (before, Windows ended the program at once and the core kept the value under test until the next logon).
- **After a BIOS update** what was learned is set aside (kept as `runs\core_knowledge.bios-*.json`) and the cores are measured again; before, values proven under the old BIOS were written back and skipped under the new one.
- **The best cores are known on processors that do not report a ranking themselves** (a 5800X3D reads 0 in both CPPC registers): the program now takes the ranking Windows uses, from the same firmware values. The window marks the best core with a gold star and the second best with a silver one (with what it means on hover), and Auto keeps its two extra steps of safety on them, as intended. Seen on the 5800X3D: core 0 gold, core 1 silver.
- At the smallest window size the button that runs a plan of one's own (Advanced, Run) is no longer cut off: the command line above it makes way when there is no room.
- A run that was stopped says so in the log ("run ended: cancelled"), not "all cores passed".
- **What a test of the held values learned is kept after each core**, not only at the end of the run (a crash late in a long run took it all).
- The window writes the file that says how Auto was started the same way as `pbo auto` (the folder is made first), and the window and `pbo auto` build the steps of the campaign in one place.
- The window: during the countdown after a crash it shows only the countdown and one button (nothing scrolls, also at the smallest size); the start button goes on at once; the hint under it no longer promises that Auto carries on after a crash when that is switched off; the cores are called "Kern" in German; a run shows how far it is ("2 von 3 Kernen").

### Not verified

This is a release, and these are its limits:

- Carrying Auto on after a crash that cuts the power or hangs the machine: what was tested is a forced restart in the middle of a run, which leaves the same traces for the program (a run that did not end, a boot time that changed).
- Everything under "Not verified" of 1.2.0-beta.1 that is not named above: a real core failure, Zen 4 and Zen 5, a run over all eight cores.

## 1.2.0-beta.1 — released as a prerelease

The Rust rebuild (`docs/rust`, `rust/`). One `pbo.exe` is the window and the command line; it takes over what a 1.x
install kept (`pbo import`). The first prerelease.

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

