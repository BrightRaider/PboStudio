# 🚀 PboStudio — CoreCycler & AMD Ryzen Curve Optimizer Studio

**PboStudio** is a stability testing and live tuning suite for AMD Ryzen processors. It combines
per-core stress testing with direct SMU Curve Optimizer read/write in a single native GUI
(.NET 9 / Avalonia UI), so you can test a value and apply it without rebooting into the BIOS.

> **Status: 1.1.0.** 271 tests, and the tuning loop verified end to end on an AMD Ryzen 7 5800X3D:
> values written to the SMU before each measurement, Prime95 pinned to a single core, a full
> stress slot, pass detection, step-down and locking at the chip limit.
>
> **Auto** runs the three phases that tuning actually takes — check what the current values
> hold, search every core that still has room, prove the result under load types the search
> never used — without asking you to come back between them, and ends with the finished values
> ready for the BIOS. It survives being stopped, and being crashed by the phase whose job is to
> find the value that crashes the machine.

---

## ⬇️ Download

| Variant | Size | What you need |
|---|---|---|
| **`PboStudio.exe`** | 45 MB | Nothing. One file, no .NET install. Fetches Prime95 and y-cruncher from their vendors on first run. |
| **`PboStudio-full.zip`** | 98 MB | Nothing, including no internet. Same executable plus both stress engines and the PawnIO driver installer. |

Take the single executable unless the machine has no internet access. Both are on the
[Releases page](../../releases).

Right-click → **Run as administrator**: talking to the SMU goes through a kernel driver, and
that needs elevation.

Windows SmartScreen will warn about an unknown publisher — the executable is not code-signed.

---

## ✨ Key Features

- 🎯 **Live Curve Optimizer control** — read and write per-core CO margins in Windows, no reboot.
- 🤖 **Auto-tuner with a two-way search** — steps each core down toward the chip limit, and on a
  failure steps it back *up*, re-testing, until a value actually holds. Only a value that has
  passed under load gets locked in.
- 🏆 **Dual-engine testing** — chains Prime95 SSE (peak boost, transient Vdroop) and y-cruncher
  (heavy vector and cache load) in one run.
- 🛡️ **WHEA watchdog** — catches corrected hardware errors from the Windows event log and maps
  them to physical cores by APIC ID. Usually the earliest sign of a too-aggressive offset.
- ⏩ **Smart recommendations** — coarse (+4) and fine (+2) mitigation proposals in one click.
- 🩺 **Platform check** — EXPO/XMP state and motherboard BIOS age, both of which affect PBO
  stability before any CO value is involved.
- ⭐ **Preferred-core badges from real CPPC data** — read from the processor, not guessed. If the
  CPU reports no ranking, no badge is shown.
- 📊 **15-minute live telemetry** — three labelled lanes (clock, temperature, PPT) with a time
  axis, the configured emergency temperature limit drawn in, and a crosshair tooltip.
- 📄 **HTML stability report** — shareable summary of values, errors and runtimes.
- 💾 **Crash recovery** — an interrupted run is detected on the next start and can be resumed.
- ⚙️ **Windows autostart task** — reapplies your tuned values on every boot.
- 🌍 **German and English**, switchable at runtime.

---

## 🛠️ Requirements

- **OS**: Windows 10 / 11 (64-bit)
- **CPU**: AMD Ryzen (Zen 2 – Zen 5, including X3D) and Ryzen Threadripper
- **Rights**: Administrator — required for SMU access
- **Driver**: [PawnIO](https://pawnio.eu/) — a signed, open-source kernel driver that restricts
  ring-0 access to a narrow set of commands. Installable from inside the app.

Prime95 and y-cruncher are downloaded from the vendors on first run, falling back to the
current version on the vendor's download page if a pinned build has been retired. PawnIO is
fetched from its GitHub release when you press Install.

The `PboStudio-full.zip` variant ships all three alongside the executable, so a machine with no
internet access can be set up completely. The app uses the bundled driver installer
automatically when it is present.

---

## ⚠️ What this software does to your computer

It changes CPU voltage offsets. Crashes and bluescreens are a normal part of finding a limit,
not a malfunction. Values applied live are gone after a reboot unless you enable the autostart
task or enter them in your BIOS.

Undervolting is not overvolting and does not typically damage hardware, but an unstable system
can corrupt data in flight. Do not tune a machine that is doing work you care about.

---

## 📘 Documentation

- [Benutzer-Handbuch (Deutsch)](docs/BENUTZER_HANDBUCH.md)
- [User Manual (English)](docs/USER_MANUAL.md)
- [Changelog](CHANGELOG.md)

---

## 🔨 Building from source

```bash
git clone <this repository>
cd PboStudio
dotnet test tests/PboStudio.Tests/PboStudio.Tests.csproj
dotnet publish src/PboStudio.App/PboStudio.App.csproj -c Release -r win-x64 \
  --self-contained true -p:PublishSingleFile=true \
  -p:IncludeNativeLibrariesForSelfExtract=true -p:EnableCompressionInSingleFile=true \
  -o out
```

`external/ZenStates-Core` is a vendored fork of
[irusanov/ZenStates-Core](https://github.com/irusanov/ZenStates-Core) with local patches;
`external/zenstates-local-patches.diff` records the difference from upstream.

---

## Known gaps

- The auto-tuner's **upward** search — core fails, voltage is raised, core is re-tested — is
  covered by tests but has not been observed on physical hardware, because the test CPU stayed
  stable down to its chip limit. The descent and locking paths are verified.
- The upward search is bounded by the configured pass count, so starting at the chip limit with
  few passes can leave a core unresolved.

### Compared with CoreCycler

Not supported, and not planned:

- **Aida64 and Linpack** engines. Prime95 and y-cruncher cover the Curve Optimizer case.
- **The old y-cruncher build** (`YCRUNCHER_OLD`), and with it the retired `N32`, `HNT` and
  `C17` algorithms.
- A **Windows Event Log** source for run and error entries. The session log is written with
  `AutoFlush`, so it survives a hard reboot on its own.
- An **update check**.
- Stress-test **process priority** and the `Debugger`/`Threads` choice for suspension.

Deliberately different:

- The stress engine is **always restarted for each core** (CoreCycler's
  `restartTestProgramForEachCore = 1`), so every core runs the same sequence. There is no
  option to carry one process across cores.
- The auto-tuner **never raises a value above 0**, where CoreCycler's `maxValue` allows a
  positive offset.
- Automatic resume after a crash is a **button, not a scheduled task**: the interrupted run is
  detected on the next start and offered with a recommendation, rather than resuming by itself
  at logon.

---

## 📜 License

GPL-3.0 for PboStudio itself. Includes a patched copy of
[ZenStates-Core](https://github.com/irusanov/ZenStates-Core), also GPL-3.0.

**Prime95** (Great Internet Mersenne Prime Search), **y-cruncher** (Alexander J. Yee) and
**PawnIO** (namazso) are owned by their respective authors and are **not covered by this
project's licence**. The single-file build downloads them from their official sources. The
offline bundle includes them unmodified and signature-intact for convenience; if you are an
author of any of them and would prefer it did not, open an issue and the bundle will be
withdrawn.

- Prime95: freeware — <https://www.mersenne.org/legal/>
- y-cruncher: freeware — <http://www.numberworld.org/y-cruncher/>
- PawnIO driver: GPL-2.0 — <https://github.com/namazso/PawnIO>; the installer wrapper declares
  no licence and is redistributed as published, signed by namazso.eu
