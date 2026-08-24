# 🚀 PboStudio — CoreCycler & AMD Ryzen Curve Optimizer Studio

**PboStudio** is a next-generation stability testing and live tuning suite for AMD Ryzen processors. It replaces text-based tools like CoreCycler by combining per-core stress testing with direct live SMU (System Management Unit) Curve Optimizer read/write controls in a single native GUI (.NET 9 / Avalonia UI).

---

## ✨ Key Features

- 🎯 **Live Curve Optimizer Control**: Read and write per-core CO margins live in Windows without rebooting.
- 🤖 **Auto-Tuner Engine & Adaptive Staging**: Automated Top-Down and Bottom-Up tuning with instant safety backoff (+3/+2) and automatic core locking (`🔒`).
- 🏆 **Hybrid Dual-Engine Testing**: Chains **Prime95 SSE** (for max boost & transient Vdroop limits) and **y-cruncher** (for heavy AVX vector & cache validation) seamlessly in a single test run.
- 🛡️ **WHEA Event Log Watchdog**: Intercepts hardware errors (WHEA 18/19) in real-time, mapping them strictly to physical core IDs.
- ⏩ **Grob (+4) & 🎯 Fein (+2) Smart Recommendations**: Instant mitigation proposals to fix instability with one click.
- 🩺 **System Health Audit**: Queries WMI for RAM EXPO/XMP speed and motherboard AGESA BIOS release dates (Score 0–100).
- 🗂️ **Clean 3-Tab Control Panel**: `[🎯 Setup]`, `[⚡ Engine]`, `[🛡️ System]` with a sticky start button, plus one place that shows all three ways a value becomes real (live SMU, BIOS list, Windows autostart) side by side.
- ⭐ **CPPC Preferred Core & Chiplet Badges**: Identifies top performing cores (🥇 Gold & 🥈 Silver). Chiplet grouping is resolved from the OS die/last-level-cache topology plus the CPU's own CCD fuse, so it is correct on 12/16-core desktop parts, Threadripper, and the Zen/Zen 2 parts that put two CCXs on one CCD.
- 📊 **15-Minute Live Telemetry**: Three separate lanes for boost clock (MHz), core temperature (°C) and PPT power (W), each with its own labelled scale, a time axis, the configured emergency temperature limit drawn in, and a crosshair tooltip with real timestamps.
- 📄 **HTML Stability Certificate Exporter**: Generates shareable, interactive benchmark reports.
- 💾 **BSOD & Crash Recovery**: Resumes interrupted test runs automatically after system reboots without losing state.
- ⚙️ **Windows Autostart Task**: Re-applies your tuned CO settings invisibly on every Windows boot.

---

## 📘 Documentation & Manuals

For complete German documentation, usage instructions, and BIOS step-by-step guides, visit:

➡️ **[Lies das vollständige Benutzer-Handbuch (`docs/BENUTZER_HANDBUCH.md`)](file:///C:/Users/Ionas/APP/PboStudio/docs/BENUTZER_HANDBUCH.md)**

---

## 🛠️ System Requirements

- **OS**: Windows 10 / 11 (64-Bit)
- **CPU**: AMD Ryzen (Zen 2, Zen 3, Zen 4, Zen 5 including X3D models) and Ryzen Threadripper
- **Permissions**: Administrator rights (Required for SMU Ring-0 hardware communication)
- **Kernel Driver**: **[PawnIO](https://pawnio.eu/)** (Signed driver, installable via built-in manager)

---

## 📜 License

GPL-3.0 License — includes patched [ZenStates-Core](https://github.com/irusanov/ZenStates-Core).
