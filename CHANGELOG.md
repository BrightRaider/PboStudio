# Changelog

## 1.1.3 — 2026-11-04

One change, found while reading the vendored library for the 1.1.2 review and not acted on then.

### Changed

- **Ryzen 7000 and 8000 (Zen 4) start at -50, like Ryzen 9000.** The library the program uses to talk
  to the processor documents the Curve Optimizer range in the commands it sends: "-30 to 30 before
  Zen 4, and from -50 to 50 on Zen 4 and newer". 1.1.2 applied that to Zen 5 only and left Zen 4 at
  -30, so a Zen 4 part that takes more was stopped at -30. Zen 3 stays at -30. The safeguards are
  the same as for Zen 5: every write beyond -30 is read back, a value the processor does not keep ends
  the search where it held, and the depth found is remembered for that BIOS. The setting under *Safety &
  system* can still hold any part at -30. The texts and both manuals say the same.
- Four existing tests asserted -30 for Zen 4 and were changed on purpose; the new tests state the rule
  (Zen 4 and newer -50, Zen 3 -30) and the Curve Shaper staying a Zen 5 feature. Run against the 1.1.2
  code before the change, eight cases of them failed with -30 where -50 is expected. 459 tests in all.

### Not verified

- **The source is a code comment in a library, not an AMD document**, and it has not been checked on
  a Zen 4 or Zen 5 processor. The read-back is what protects a board that takes less.
- Everything listed as unverified under 1.1.2 still is: the Zen 4 telemetry positions, the write path
  beyond -30, the core mapping on 6+6 and 6-core parts, and the APIC mapping where IDs skip numbers.

## 1.1.2 — 2026-11-04

A review of the whole code base, not of a diff, after a run of defects that each turned out to be
somewhere nobody had looked. It found more than expected, several of them in the part of the program
that decides whether a core passed. 453 unit tests, up from 318. For the runner fixes and the log reading,
the new tests were run against the previous code and fail there; the others are covered by tests of the
new behaviour, and the ones that depend on real hardware say so below.

### Fixed — a core could pass without being tested

- **A refused deep value was booked as a pass.** When the processor will not take a Curve Optimizer
  value below -30, the runner tested "whatever the CPU currently holds" and recorded the result against
  the value that had been asked for. On a board that stops at -30 the search walked to its floor on
  passes at a depth that never ran and locked a figure nobody tested. A refused value now ends the
  search at the last value that held, with the usual headroom; a core with nothing proven to fall back
  on is skipped rather than tested. Inside -30 nothing changed: that path is the verified one.
- **A failure line split across two polls was missed.** Both engines report an error by writing a line
  that the program reads once a second. The read went to the end of the file and resumed from the
  file's length, so a line that was half written at that moment had its first half inspected and its
  second half never. "FATAL ERROR" cut between two polls matched neither, and the core passed. Only
  complete lines are consumed now.
- **Cores 6 to 11 of a 12-core processor were addressed on the wrong chiplet.** The map from Windows
  cores to the SMU's (chiplet, slot) addressing divided by eight - the library's slot count - where it
  needed the number of enabled cores. It is right for an 8-core and a 16-core part and wrong for every
  6+6 and 6-core one (7900X3D, 7900X, 5900X, 9900X, the 6-core parts): a value went to some other
  core, the core under test passed at a value it never held. The fuse map the processor reports now
  decides, and the mapping is written to the log at start. **Not run on such a processor** - it is
  covered by tests on synthetic fuse maps, and on the 8-core part this was verified on it produces the
  identical mapping, which was checked on the real hardware.
- **A machine check could be blamed on the wrong core.** The APIC ID in a WHEA event was taken to be the
  logical processor's number. True where it was checked, an assumption where cores are fused off and the
  IDs skip numbers - and the error runs in the dangerous direction: a genuine fault on the core under
  test credited to another core and reported as "not counted against this core". The IDs are read from
  the processor itself (CPUID leaf 0xB on each logical processor), which on the 8-core part came back as
  the identity.
- **The crash check could be crowded out.** It looked at the twenty newest WHEA records of any kind, so a
  few dozen corrected errors after a crash pushed the crash out of view. Fatal events are queried
  directly.

### Fixed — values lost, or recorded wrongly

- **Stopping a run left the other cores parked at 0.** With "test only this core" the others are held at
  0 for each slot and restored at the end - but only on the normal way out. Stopping, the temperature
  cut-off, an engine that would not start, and closing the window all skipped it, leaving the undervolt
  gone from every other core while the table showed it. The next start then read those zeros as the BIOS
  values. Restoring is unconditional now, and closing the window stops the run first.
- **A closed program was recorded as a system crash.** Any run that ended abnormally left its state file,
  and the next start reported "unexpected system restart on core N" and wrote that core's value into the
  tuner's memory as a failure for good. A real crash is now told apart by the computer having started
  again after the file was last written.
- **"Apply live" showed a value the processor had not kept.** The typed number was marked applied and
  saved to the autostart profile. A write beyond -30 is read back and the row shows what the processor
  holds.
- **The autostart profile was written to the processor unexamined at every logon.** It is checked now:
  it must belong to this processor (a 5800X3D swapped for a 7800X3D has the same eight cores and would
  have received every old value) and every value must be in range.
- **Combo boxes restored to the wrong item.** The order box has five entries and was clamped to three, so
  "core pairs" came back as "custom order"; free y-cruncher algorithm choice was lost the same way.
- **"Back off and resume" did something other than its label.** The button said -24 and applied the value
  the processor held after the reboot (the BIOS value) plus four.
- **A second run could start beside the first** while the restore point was being created, which can
  take minutes. And a campaign start that bailed out (no engine, setup window opening) was recorded as a
  run, so pressing Continue afterwards was met with "the last run changed nothing".
- **A finished run kept spinning on pair orders**, and a custom order over a subset of cores ended after
  its first pass regardless of how many were asked for.

### Fixed — security

The program runs as administrator and starts what it downloads.

- **The PawnIO installer - a kernel driver - was started elevated with no check.** It sat under a fixed
  name in the shared temp folder, where it could be planted in advance, and nothing verified the bytes.
  It is downloaded to a private folder and checked against its SHA-256 before it runs; the copy in the
  offline bundle is checked the same way, and the pinned hash was confirmed against the one GitHub
  publishes and against the bundled file, which match.
- **A plain-http address stood in the y-cruncher mirror list**, the page scanned for the current version
  was fetched over http, and a link found there was followed whatever host it named. Everything is https
  now, and a download is only taken from the vendor's own domain.
- **PowerShell, schtasks, shutdown and rundll32 were started by bare name**, which looks in the program's
  own folder first. They are started from the system folder by full path.
- **A hung PowerShell froze the caller for good.** Output was read to the end before the timeout was
  consulted; reading to the end blocks until the process lets go. This was the call that creates the
  restore point, on the thread that starts a run.
- The webhook only accepts https addresses and gives up after fifteen seconds.

### Fixed — smaller

- **Durations past a day were wrong.** "h:mm" on a TimeSpan prints the hours component, 0 to 23: the
  elapsed counter jumped back to zero after a day, and an estimate of 24 hours read as 0:00. Fourteen
  places.
- **The platform audit invented a fault.** When the memory speed could not be read it substituted values
  that read as "EXPO is off" and took 25 points off. An unreadable value is now unknown, not a finding.
- **The report** wrote the processor and profile names into the page unencoded, and its footer named a
  version, "v2.0", that no release has had.

### Changed

- **Ryzen 9000 starts at -50 again.** It was the default from the first commit, was removed in 1.1.0 on
  an unsourced claim, and is back, with what was missing then: every write beyond -30 is read back, a
  value the processor does not keep ends the search where it held, and the depth found is remembered for
  that BIOS, so a board that stops at -30 costs one refused write per core, once. Every other part keeps
  -30 as its starting point. The setting under *Safety & system* can now hold a Ryzen 9000 at the standard
  as well as deepen any part. It is stored as a choice or as no choice - a stored -30 used to be
  indistinguishable from an untouched box and would have pinned every Ryzen 9000 that ran 1.1.1.

### Not verified

- **The Zen 4 telemetry positions** (1.1.1) and the **deeper-than--30 write path** have still not run on a
  Zen 4 or Zen 5 processor.
- **The core mapping on 6+6 and 6-core parts**, and **the APIC mapping where IDs skip numbers.** Both are
  tested against synthetic maps; both gave the identity on the 8-core part. The logic is the same either
  way, but neither has been seen on the processors it is for.
- Zen 5 telemetry is still "n/a": its power table is not recognised.

## 1.1.1 — 2026-11-04

Two reports from Ryzen 9 7950X3D owners, both of which turned out to be this program's own
mistakes. 318 unit tests.

### Fixed

- **Temperature and power read 0 on Zen 4.** The live readings come out of a power table the
  SMU publishes into memory, and the program read one fixed set of positions out of it — the
  Zen 3 ones — on every processor. On a Zen 4 part those positions hold something else. Each
  table version now has its own layout, and a table the program does not recognise shows "n/a"
  instead of a zero it did not measure.

  **The Zen 4 layout is a best reading, not a verified one.** Its positions come from the sensor
  definitions in the bundled ZenStates-Core library, not from a measurement made here, because
  no Zen 4 processor was available. The Zen 3 layout is the one checked on hardware, and it is
  unchanged. For any layout that has not been checked, and for any table that is not known, the
  program writes the SMU's own dump of it to `runs/pm_table_report.txt` and says so in the log,
  so that the right positions can be found by comparing it against another monitoring tool.

- **The temperature cut-off could not act, and nothing said so.** It compares a run against the
  same reading the display shows. With no reading it never fires: a stress test ran with no
  thermal protection on a processor whose table was misread. A run now warns, once, when there
  is no temperature to act on. A temperature outside any believable range is discarded rather
  than trusted, because a misread low figure is a false sense of protection and a misread high
  one only ends a run early.

- **A core the BIOS set deeper than -30 was shown as -30.** Each row clamps to the depth the
  program allows, and the program allowed -30. A core holding -40 was displayed as -30, counted
  as edited, and "Apply live" would have written -30 over the BIOS value. A value the processor
  reports holding is now taken as proof that it accepts it.

### Changed

- **The Curve Optimizer depth is no longer a constant.** 1.1.0 said that -30 is the limit on
  every AMD desktop part. That was stated as a fact and was not sourced, and it is contradicted
  by a Ryzen 9 7900X3D / 7950X3D owner who uses -40. There is no AMD document that settles it;
  what exists is forum reports of ±50 on newer AGESA versions and a quoted comment that the
  range was extended with the 7000X3D. The earlier -50 default was just as unfounded, applied to
  everybody — so the default stays at the standard -30, which is what every part is known to
  accept, and the depth actually used is worked out from evidence: what the processor reports
  holding, what the person asks for, and where a write stops sticking.

  A new setting under *Safety & system*, "Deepest Curve Optimizer value", lets a person say
  their BIOS accepts more, down to -50. Every write beyond -30 is then read back. A value the
  processor does not keep is reported and is not tested as though it had been set, which would
  have let the search lock a figure that never ran.

  Inside -30 nothing is second-guessed. That path is the one verified on hardware, and a
  mismatch there is more likely a read that raced the write than a refused value.

### Not changed, and why

- **Zen 5 telemetry.** Its power table is not recognised, so it shows "n/a" and the dump is
  written. Reading it wrongly would be worse than not reading it.

### Known gaps

- The Zen 4 telemetry positions and the deeper-than--30 write path have not been run on a
  Zen 4 or Zen 5 processor. Both are covered by tests against synthetic tables and values.
- The 1.1.0 entry below still states the -30 claim as it was written. It is left as history;
  this entry is the correction.

## 1.1.0 — 2026-09-18

One button that runs the whole thing, a window cut down to three tabs, and a Curve Optimizer
range that was wrong on every AM5 processor. 271 unit tests.

### Added

- **Auto.** Tuning a Curve Optimizer is three runs — establish what the current values do,
  search every core that still has room, then prove the result under load types the search
  never used — and the program used to ask the reader to come back between them, read a card
  and press a button again. That is not a tool that tunes a processor; it is a tool that tells
  you what to do next. Auto chains all three and ends with the finished values, formatted for a
  BIOS.

  What makes it safe to leave alone: the decision of what to run next is the one
  `NextStepService` already made and already had tests for. On top of that sits when to stop
  (a confirmation run that passes is the end, not another prompt), how to notice that nothing
  is moving (every failure raises a core's floor and every pass lowers what it holds, so two
  identical signatures in a row mean the run learned nothing and would learn nothing next
  time), and automatic back-off — a core that failed still holds the value it failed at, which
  would send the campaign back into the same run forever.

  The campaign is on disk. Phase two exists to find the value that reboots the machine, so it
  has to survive that, and stopping it deliberately works the same way: it resumes at the same
  phase.

- **A runtime estimate above the start button, on every tab.** Worked out from what each core
  currently holds — read from the processor at start-up, so it reflects the BIOS settings in
  force — and from what is on record about it. Cores already parked where the search would
  leave them cost nothing in the search phase, which is the difference between an evening and
  three days. Past a day it reads "1 day 4 hrs" rather than 28:00.

- **The four CoreCycler settings that were missing.** `flashOnError` (the taskbar flashes on
  each failure as it happens, not only in the summary — by the time a run ends the interesting
  failure may be six hours old), `stressTestProgramPriority`, Prime95's `TortureMem` (it was
  written into the config as a constant zero, which is right for Curve Optimizer work but is
  also the setting that turns this into a memory controller test), and
  `treatThreadErrorsAsRealErrors` as "count a worker that died or went quiet as an error". A
  worker can vanish because an antivirus took it or Windows killed it under memory pressure,
  and counting that means backing a good core off for nothing. A wrong calculation and a
  machine check are never filtered by it.

### Fixed

- **The Curve Optimizer range was wrong on every AM5 part.** The limit table let a Ryzen 9000
  be walked down to -50, on the reading that Zen 5 has an extended range. It does not: Curve
  Optimizer is -30 to +30 on every AMD desktop part that has it, and what Zen 5 added is Curve
  Shaper — a second, separate set of offsets across temperature and frequency bands, which this
  program does not write. A search allowed to run to -50 ends by reporting values that cannot be
  typed into a BIOS, which is the whole deliverable.

- **A campaign could never reach phase three.** A core counted as still having room when it
  held a value milder than its floor, but the search deliberately locks each core at floor plus
  the guardband, which is milder than the floor by construction. Every core the search had just
  finished still looked open. A simulation of the whole campaign against a fake machine found
  it in three scenarios out of three.

- **Half the search estimate was a climb that cannot happen.** A core with a value on record
  that held cannot be walked back up from scratch — a failure returns to that value and locks
  there without retesting it. Costing it as though it might climb turned a half-hour phase into
  two and a half hours. It hit exactly the cores a campaign is usually resumed on.

- **Two Cancel buttons.** The footer's single-run button was hidden on the Auto tab only while
  idle, so starting a campaign put two of them on the same panel.

- **The collapsed telemetry dock clipped its own tab strip.** A fixed 46 pixels is a couple
  short of the strip plus the card's padding, so "Live telemetry" and "Log" were cut off along
  the bottom edge of the window. The row measures itself now.

- **The headroom field rendered as "+" with no number**, because it sat beside its label in 92
  pixels and the two spinner buttons left nothing for the text.

- **"Which Zen is this" had three answers that disagreed** — a detector for the y-cruncher
  binary, the limit table, and a third list in a recommendation service. They read one model
  now, covering every AM4 and AM5 line including the APUs, which break the "leading digit is
  the generation" rule in both directions. It also knows that Curve Optimizer arrived with Zen
  3: on Zen 1, Zen+ and Zen 2 the Auto tab says so instead of spending a weekend measuring a
  setting that does not exist, and that the Zen 3 X3D parts take an undervolt only.

### Changed — the window

- **Three tabs: Auto, Tests, Advanced.** The previous arrangement put one Start tab above two
  expert tabs, which made the expert tabs look like steps two and three of something.

- **Tests is four things and a button**: which profile, whether to show all fourteen, what that
  profile runs, how long it takes. It carried a recommendation card that answered the same
  question as the picker below it, with a second start button and a second copy of the runtime
  — refreshed on different events, so the two drifted apart and at least one was always wrong.
  They had reached a factor of eight.

- **The profile list shows five entries, not fourteen.** The five are the path: first run,
  narrowing, a short look, the game-style failure continuous load never produces, and the final
  proof. The other nine are variations, one checkbox away. Profiles are stored by identity now,
  because an index into a list that can be filtered does not mean the same profile twice.

- **Advanced is grouped.** Ninety-odd controls in the order they happened to be written is not
  a settings page. Five headings, in the order somebody works through them, and one line at the
  top saying the thing that makes the rest make sense: everything here overrides the profile
  chosen under Tests.

- **The auto-tuner is the step search**, and lives on Advanced. It sat on the Tests tab one tab
  away from a tab called Auto, which is two different things sharing a word.

- **Prose that explains a choice already made moved into tooltips.** What is left on screen are
  the facts: what runs, how long, what the plan is.

- **PPT, TDC, EDC and the scalar only appear while something is running.** At idle they are four
  chips of jargon reporting that nothing is happening.

- **Controls the plan captured at start are disabled during a run.** Changing one did nothing,
  except the tuner's memory reset, which moved the floor out from under a search that was using
  it. The maximum temperature stays live on purpose: raising a safety limit must never require
  stopping first.

### Removed

- The old recommendation service. Nothing referenced it, and it still named profiles by display
  string — the drift that ids were introduced to stop — pointing an X3D at AVX2, the hottest
  load in the set on the design least able to take the heat.

### Testing

271 unit tests, up from 164. The ones worth naming are the ones that replace looking at the
window: a simulation that runs a whole campaign against a fake machine and has to reach the
end, a record of which parts of the Auto tab are visible in which state, and a set that reads
the markup for controls named twice, handlers without methods, translation keys without
entries, and anything that drifted back into the wrong tab. Every one of them corresponds to
something that shipped broken during this release.

### Known gaps

- Aida64 and Linpack are still unsupported as engines. Neither is a Curve Optimizer engine and
  Aida64 is paid.
- The estimate is an upper bound. The search stops early once a value holds, so a run usually
  finishes short of it.

## 1.0.2 — 2026-08-25

A UI/UX audit and a feature-by-feature comparison against CoreCycler v0.11.0.3, with the
findings from both fixed. 164 unit tests. The interface changes have not been seen on screen
yet — the hardware run that shook out the engine side was made with an earlier build of this
release, before the toast layer, the collapsible dock and the auto-tuner memory existed.

### Fixed

- **Two y-cruncher algorithm tags did not exist.** Every preset passed `SFT` and `FFT`, the
  v0.7 spellings. y-cruncher 0.8.x renamed them to `SFTv4` and `FFTv4` and does not alias the
  old names — its own `Command Lines.txt` lists the surviving aliases explicitly and these two
  are not among them. Tags go straight onto the command line, so this failed quietly: the
  recommended Curve Optimizer preset was asking for two tests by names the binary no longer
  knows. A test now checks every preset against the documented tag list.
- **A machine check from any core was charged to the core under test.** WHEA events name the
  logical processor that raised them and the mapping already existed, but the runner counted
  every event against whichever core happened to be running. A core could be backed off for a
  fault it never had. Events from elsewhere are now reported and not counted.
- **The table header did not sit above its columns.** Header and rows both declared an `Auto`
  first column while only the rows pinned a width, so on a single-CCD processor — a 5800X3D, a
  7800X3D, a 9700X — the "Curve Optimizer" heading sat about 42 px left of the column it
  labelled. Both now size from one value.
- **Warnings were invisible.** Everything the program had to say went to the log tab, which is
  closed on startup. "No core selected", "nothing to do — no value was changed" and, worst,
  "SMU unavailable" — the only explanation for why *Apply live* is greyed out — looked from the
  outside like the button not working. Anything at warning level or above now appears on screen.
- **The telemetry lane titles were drawn over their own curves.** The plot area started at the
  top of each lane, so any value near the maximum ran straight through the text naming it.
- **The core table stayed editable during a run**, accepting selection and margin changes the
  run then ignored, while every other control was locked.
- **The platform check inherited the failure styling.** After a failed run it appeared in the
  red panel, reading as a second failure.
- **Seven combo box entries were never translated.** `ModeOpt0-3` and `YcAlgoOpt0-2` carried
  `x:Name` attributes but no assignment, so a German UI showed "SSE (Recommended for CO)" and
  "Comprehensive (All 8 Algorithms)" permanently.
- **The three apply hints were trimmed to an ellipsis** at any normal window width, hiding
  exactly the part that says what each option costs ("permanent · scheduled task · needs admin").
- The post-recommendation message pointed at the bottom **right**; the apply block is below the
  core table on the left.

### Added — the failure a stress test cannot produce

A Ryzen 7 5800X3D passed hours of Prime95 and y-cruncher, then hard-reset during an ordinary
game. Windows logged two fatal WHEA 18 cache-hierarchy errors in the same second, APIC 0 and
APIC 9 — physical cores 0 and 4 — with the uncorrected and context-corrupt bits set. One of
those cores was sitting at -20, nowhere near its limit.

That is the gap: a continuous full load holds the core at a lower boost clock, while a
light-load game swings it to its highest single-core frequency, where a negative offset has the
least voltage left to give. The test never visits the state that fails.

- **Headroom when locking.** The auto-tuner used to lock the boundary margin itself — the exact
  value at which the core just barely held under that particular load, with nothing in reserve
  for temperature drift or a load pattern the test never produced. It now locks a configurable
  number of points above it, default 3. The measured boundary is still recorded, and the advice
  line names both so the difference never looks like a mistake.
- **Preferred cores get more.** The cores CPPC ranks highest boost furthest and carry the
  Windows background work, so they get two extra points on top.
- **A micro-burst profile.** Sub-second pulsing — hundreds of transitions a minute instead of a
  dozen — driven by its own timer rather than the runner's one-second tick. It runs the
  *coolest* load in the set, SSE on one thread, on purpose: heating the core with AVX2 would
  pull the clock away from the state that fails. The occupancy check now measures against the
  configured duty cycle, or every micro-burst slot would be reported as a core gone idle.
- **Crashes that happen outside a test run are picked up.** The auto-tuner only ever learned
  from its own slots, which is the smaller half of the evidence; a crash while gaming left its
  only trace in an event log nothing read. Fatal machine checks are now read at startup, mapped
  to cores by APIC id, and offered for recording. Offered, not written: values applied live are
  lost in the reboot, so the margin a core holds afterwards may be milder than the one that
  failed, and recording that on a guess would cap the core far too tightly.

### Added — the auto-tuner remembers

- **A core that crashed the machine is never walked back into that value.** The search state
  lived only in memory, so the one run that learned something — the one that ended in a crash —
  was exactly the run that forgot it. And nothing survived between sessions at all: the sole
  record that a core had taken the system down at a given margin was a line in a log file no
  code reads, so the next run would descend straight back to it.
  `runs/core_knowledge.json` now holds, per core, the most aggressive margin it has ever
  survived and the mildest one it has ever failed at. The auto-tuner takes its **floor per
  core** from that instead of applying one chip-wide limit to all of them, a crash is recorded
  as the failure it is, and the core table marks the affected cores with ⛒ and explains the
  floor in the tooltip.
- **The memory can be discarded** from the auto-tuner panel, with a confirmation that spells
  out the consequence. Worth doing after a BIOS/AGESA update, a changed memory profile or new
  cooling.
- **The BIOS version is stored with the memory**, and a mismatch is flagged. An AGESA release
  changes boost behaviour and voltage curves, so observations from the old firmware may describe
  a machine that no longer behaves that way. Reported, never acted on — the reader decides.

### Added — from the CoreCycler comparison

- **Automatic runtime.** A core is held under load until the engine has worked through its
  whole list once, rather than for a fixed six minutes — which on a preset like Huge only gets
  through a fraction of the FFT sizes. Prime95 reports a completed sweep by repeating an FFT
  size, y-cruncher by its iteration headers; both are bounded by a configurable cap, because
  detection reads the engine's own output and must not be able to pin a core indefinitely.
- **A system restore point before the auto-tuner.** The auto-tuner deliberately drives cores
  past stability and an unstable core does not always fail cleanly. On by default.
- **Park other cores while testing one** (CoreCycler's `setVoltageOnlyForTestedCore`), with the
  margins restored when the run ends.
- **Spread one worker across both SMT siblings** — a third option between pinning one thread
  and saturating both, which produces transitions neither of those creates.
- **Core-pair ordering**, for instabilities that only appear on a specific hand-over.
- **Free y-cruncher algorithm selection**, plus per-algorithm duration, memory, and manual
  binary choice — the binary decides the instruction set, so testing both a cold and a Zen-native
  build is a real strategy that was previously impossible.

### Changed — one recommendation instead of thirteen options

- **The card covers the whole journey, starting from an empty machine.** It used to assume the
  program was ready to run, so somebody opening PboStudio for the first time — no driver, no
  stress engine — was told to run Heavy FFTs, next to a start button that was greyed out. The
  first two steps are now the driver and an engine, each saying what the thing is *for*:
  "install PawnIO" means nothing to a person who has never heard of it. The card carries a
  warning tint while anything is missing, hides the profile it cannot run, and its button opens
  the assistant instead.
- **The first run states that it changes nothing permanently.** A newcomer pointing a tuning
  tool at their own processor deserves to be told that up front, not to infer it.
- **The Setup tab leads with a single answer.** Thirteen profiles are thirteen answers to a
  question nobody asked; what a person wants to know is what to run *now*, and at any moment
  that has one correct answer. A card names it, says why in terms of this machine, and sets the
  whole run up in one click — profile, auto-tuner, core selection and pass count. The full list
  is still there, one click away under "Choose a different profile".
- **The first run alternates both engines, as it should.** The recommendation named the
  single-engine Heavy FFTs profile while the profile list's own entry #0 — labelled 🏆 and
  described as "the gold standard from CoreCycler" — is the Prime95 SSE → y-cruncher chain.
  Validating a Curve Optimizer setting means passing both: a core can sail through Prime95 and
  fail y-cruncher's cache and memory pressure. CoreCycler needs two runs and its multiconfig
  launcher for this; the combined profile chains them in one. The confirmation run already
  covered both.
- **Except while the auto-tuner is searching**, which stays on one engine — and now says why.
  Locked cores are shared across phases and excluded at the start of each, so a core the tuner
  settles during the Prime95 phase never enters the y-cruncher phase: the second engine would
  cost the full runtime and measure nothing. Both load types belong on the confirmation run,
  where nothing is locked.
- **The recommendation was wrong for X3D parts.** It mapped CPU generation to a profile and,
  for Zen 3 X3D, named the AVX2 profile: the hottest load in the set, on the one design whose
  stacked cache is most sensitive to temperature — contradicting the project's own X3D guidance
  elsewhere. It now opens with SSE on every part and says why.
- **Recommendations no longer match profiles by name.** The service named its target as a
  display string and the UI matched it back by counting shared words, because the two lists
  were maintained separately — and they had already drifted ("Full Run - SSE & AVX2 Combined
  (Recommended)" against the actual "Full Run - Prime95 SSE & AVX2"), so the right profile was
  being found by luck. Profiles now carry a stable id.
- **The recommendation knows where you are.** It reads the core memory: nothing measured yet
  means start with the workhorse; cores still holding a milder value than they could reach
  means point the auto-tuner at *those cores only*; nothing left to search means run the
  overnight confirmation. Testing two cores instead of eight is the difference between an
  evening and a night.

### Changed — interface

- **The three tabs are no longer equal.** `[Setup] [Engine] [System]` read as three steps to
  work through. In practice the first one is the whole application — a profile carries the
  entire configuration — and the other two are expert surfaces that override it. Setup now
  leads on its own row as "Profile & start"; Engine and System sit below it, quieter, under an
  "Advanced" label. Nothing was removed; the weighting just stopped lying.
- **The graph and log dock collapses.** At 150 % Windows scaling its fixed height was the
  difference between five and ten visible cores on a 16-core part. Expanded it is now taller, so
  three telemetry lanes and their labels actually fit.
- **Contrast.** `TextFaint` carried the apply hints, profile help, metric labels and delta
  column at 3.5–4.0:1, below AA. The graph's axis numbers were at 2.63:1 — the figures that make
  the curves readable were effectively invisible. Both raised, and the smallest type step with them.
- **The auto-tuner shows its search.** Each core now carries the values it tried and how they
  went (`-25 ✕ · -22 ✕ · -19 ✓`) instead of one aggregate "n of m cores locked".
- **Curve Optimizer values show their sign.** `+5` displayed as `5` in a column that also holds
  `-30`.
- **The risk bar has a second channel** — a glyph past 70 % — because green against orange at
  two pixels is the pairing red/green colour blindness cannot separate.
- **Profiles show the five shelves they were always sorted into**, which existed only as source
  comments.
- **Focus is visible on input controls.** The existing rule only reached buttons, leaving the
  Curve Optimizer fields — the most keyboard-driven part of the window — without one.
- The setup assistant **numbers its three steps**, names the next action, and offers to restart
  PboStudio after PawnIO rather than describing that it must be.
- Temperature leads the metric strip; it is the only value there that ends a run.
- `Ctrl+C` is back to copying the selection; the BIOS list moved to `Ctrl+Shift+C`.
- Minimum window height lowered from 560 to 480.

## 1.0.1 — 2026-08-25

Verified on hardware. An AMD Ryzen 7 5800X3D ran the full loop: the margin written to the SMU
before each measurement, Prime95 pinned to one core, a complete stress slot, pass detection,
step-down to the chip limit and locking there.

### Fixed

- **The auto-tuner forgot everything between phases.** A profile like Hybrid Ultimate runs
  several phases, each with its own runner, and both the per-core search state and the set of
  locked cores lived inside it. A core that failed at -28, climbed to -25 and locked there would
  be tested again in the next phase with no memory of any of it — pass at -25, look like a fresh
  descent, and be stepped straight back down to -28. The lock icon stayed on screen throughout.
- **Engine downloads were pinned to one version each, and both had 404'd.** First-run setup was
  a dead end. Downloads now try pinned builds, then discover the current archive from the
  vendor's own download page, and only then fail — with the page and target folder named, and a
  button to open it. A payload that is not a ZIP is rejected rather than handed to the extractor.
- **The offline bundle was not offline.** It carried both stress engines but still needed the
  network for PawnIO, without which the app cannot reach the SMU at all.

### Changed — profiles

- **A profile is now the whole configuration.** Each one carries its engine, workload, runtime,
  thread count and transient-pause behaviour, and states what it runs directly under its name
  ("Prime95 SSE · Huge → y-cruncher · 6 min/core × 3, load transition every 30s"). Picking one
  is a single decision again.
- **The engine selector no longer overrides profiles silently.** It defaults to "As the profile
  says"; forcing Prime95 or y-cruncher is still possible but is now a deliberate act. Before,
  the box sat on "Prime95" out of the box and quietly turned "Recommended - Prime95 SSE &
  y-cruncher" into neither.
- **Transient pauses belong to the profile.** The one setting that most reliably breaks a
  borderline Curve Optimizer value was the only one no profile carried, so it always had to be
  dialled in by hand.
- **New profiles**: `🔥 Breaking point - hard load transitions` (AVX2, smallest FFTs, both
  threads, a load transition every 5 s — the hardest thing the tool does), `Heavy FFTs - the
  CoreCycler classic`, and two y-cruncher-only profiles. The overnight run now ends with a
  y-cruncher phase.
- **FFT ranges match CoreCycler**, including its Heavy, HeavyShort and Moderate presets, so
  results are comparable between the two tools.

### Added

- The Setup tab states the phases a run will actually consist of. A profile's first phase is
  replaced by the Engine tab's instruction set and FFT range while later phases are not, so a
  profile promising "SSE with huge FFTs" could genuinely run small ones with nothing on screen
  saying so.
- `PboStudio-full.zip`: executable, both engines and the PawnIO installer, for machines with no
  internet access. CI fails the build if the driver installer's signature is not valid.

---

## 0.9.0 — 2026-08-24

First versioned release. The project had no version history before this point; everything
below was found and fixed in a single review pass.

### Fixed — correctness

- **The auto-tuner tested values the processor never held.** `TestRunner` tracked margins
  internally but never wrote them to the SMU before measuring a core. Entering `-30` and
  starting the tuner tested the BIOS value instead, then locked `-30` in as "validated"
  although it had never been under load. `TestPlan.ApplyMargin` now writes the planned value
  immediately before each measurement, and logs every write.
- **The auto-tuner guessed instead of searching.** After a failure it added a fixed `+3`/`+2`
  and locked the core — a value nobody had tested. It now walks back up, retesting, until a
  value actually holds; only that one is locked. A core that already passed at a higher value
  settles there without a retest. A core failing at `0` reports that the cause is not the
  Curve Optimizer.
- **"Reset" stopped returning to the BIOS values.** One field served as both "BIOS baseline"
  and "last applied", so every write silently redefined what reset meant — and after an
  auto-tuner run it did nothing at all. The two are now separate.
- **Startup crashed without PawnIO.** `SmuService.MaskFor` divided by zero when the topology
  reported no cores, instead of degrading to the stress-test-only mode the UI advertises.
- **The telemetry graph was invisible.** As a `DockPanel` sibling it took the implicit
  `Dock=Left` with zero desired width.
- **The Accept button could disappear permanently.** One click on "Copy BIOS list" or the
  platform check hid it, and nothing ever showed it again — so the recommendation from a
  failed run could not be applied.
- **Chiplet grouping was arithmetic guesswork** that only fitted 12- and 16-core desktop
  parts. It is now resolved from the OS die/last-level-cache topology plus the CPU's own CCD
  fuse, and handles Threadripper and the Zen/Zen 2 parts that put two CCXs on one CCD.
- **`Ctrl+C` was captured globally**, so selecting log text and copying it produced the BIOS
  list instead. `Space` no longer starts a run either — it fired while a ComboBox had focus.
- **Preferred-core badges were invented.** `CoreQualityService` hard-coded core 0 as gold and
  presented it as a CPPC measurement. It now reads the values the processor reports, and
  shows no badge at all when they are unavailable.

### Added

- Custom FFT range and custom core order, both already supported by the engine layer but
  previously unreachable from the UI.
- A session log file under `runs/logs/`, written through on every line so it survives a
  bluescreen, with severity colouring in the UI and a folder shortcut.
- Settings persistence in `runs/settings.json`: language, profile, engine parameters,
  temperature limit, webhook, window size.
- Controls for `SkipCoreOnError`, `TreatWheaWarningAsError` and `DelayBetweenCores`, which
  existed only as hard-coded defaults.
- Dependency check on startup: missing components are named before the start button is
  pressed, and the button stays disabled until a stress engine is present.
- Confirmation before setting every core to the chip limit.

### Changed — interface

- Shared design system in `Styles/Theme.axaml`; both windows draw from one set of tokens.
  Type scale reduced from thirteen sizes to five, corner radii from five to three.
- The right-hand tab body scrolls. It could not display its own content at the default window
  size, and the progress panel was the part that got clipped during a run.
- The three ways a value becomes real — live SMU, BIOS list, Windows autostart — sit together
  under the core table with their trade-offs stated.
- Core rows are 33px instead of 47px; the window sizes itself to the work area.
- Telemetry redrawn as three labelled lanes with a time axis, a 15-minute window (was two
  minutes) and the configured temperature cut-off drawn in.
- The setup wizard is scrollable, resizable, localised, and can install everything missing in
  one action.
- Localisation is complete in both directions. Status text, results, the wizard, the platform
  check and every tooltip previously appeared in German regardless of the selected language.

### Known gaps at the time

- No end-to-end run had been verified on real hardware. Resolved in 1.0.0.
- Aida64 and Linpack engines are not supported; CoreCycler has them.
- The auto-tuner's upward search is bounded by the configured pass count.
