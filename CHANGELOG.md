# Changelog

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
