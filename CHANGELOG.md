# Changelog

## 1.0.2 — 2026-08-25

A UI/UX audit and a feature-by-feature comparison against CoreCycler v0.11.0.3, with the
findings from both fixed. 145 unit tests. The interface changes have not been seen on screen
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
