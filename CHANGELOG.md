# Changelog

## 0.9.0 — 2026-08-24

First versioned release. The project had no version history before this point; everything
below was found and fixed in a single review pass, so it is grouped by kind rather than by
release.

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

### Known gaps

- **No end-to-end run has been verified on real hardware.** The test suite covers the logic;
  the full path — engine launch, core pinning, failure detection, SMU writes under load — has
  not been exercised.
- Aida64 and Linpack engines are not supported; CoreCycler has them.
- The auto-tuner's upward search is bounded by the configured pass count. Starting at the
  chip limit with few passes can leave a core unresolved.
