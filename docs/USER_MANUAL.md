# 📘 PboStudio — Master Manual & Complete Documentation

> **PBO Studio**: The All-in-One Stability Testing & Live Curve Optimizer Suite for AMD Ryzen Processors.

---

## 📋 Table of Contents
1. [About the Project](#1-about-the-project)
2. [Prerequisites & PawnIO Kernel Driver](#2-prerequisites--pawnio-kernel-driver)
3. [User Interface Overview](#3-user-interface-overview)
4. [System Health Audit (EXPO RAM & AGESA BIOS Check)](#4-system-health-audit-expo-ram--agesa-bios-check)
5. [WHEA Watchdog & Smart Recommendations (Coarse vs. Fine)](#5-whea-watchdog--smart-recommendations-coarse-vs-fine)
6. [The Auto-Tuner Engine (Automated CO Step-Down)](#6-the-auto-tuner-engine-automated-co-step-down)
7. [Applying CO Values Live vs. Motherboard BIOS Flashing](#7-applying-co-values-live-vs-motherboard-bios-flashing)
8. [HTML Stability Certificate Exporter](#8-html-stability-certificate-exporter)
9. [Crash Protection & BSOD Recovery](#9-crash-protection--bsod-recovery)
10. [Frequently Asked Questions (FAQ)](#10-frequently-asked-questions-faq)

---

## 1. About the Project
**PboStudio** combines true per-core stress testing (popularized by tools like CoreCycler) with direct live control of the AMD Curve Optimizer via the **SMU (System Management Unit)** in a single native desktop interface.

No manual `.ini` file editing is required — testing, error analysis, and core voltage adjustments occur in real-time within a unified workspace.

---

## 2. Prerequisites & PawnIO Kernel Driver
- **Operating System**: Windows 10 / 11 (64-Bit)
- **Processor**: AMD Ryzen (Zen 2, Zen 3, Zen 4, Zen 5 including X3D variants)
- **Permissions**: Administrator Privileges (Required for Ring-0 SMU hardware access)
- **Kernel Driver**: **PawnIO** (Digitally signed Windows driver; eliminates unsafe legacy WinRing0/inpoutx64 drivers). Installable with 1 click via the built-in *⚙️ Driver & Engines Manager*.

---

## 3. User Interface Overview

- **Top Header Bar**:
  - **Left Side**: Application brand logo, detected CPU model, and live CPU telemetry (Boost Clock, PPT Power, TDC Current, EDC Current, Core Temperature).
  - **Right Side**: Clickable `🔍 SYSTEM: xx/100` Audit Button, `🌐 EN / DE` Language Switcher, `⚙️ Driver & Engines Manager`, and isolated `🛡️ WATCHDOG` alert badge.
- **Left Column (Core Table & Recommendations)**:
  - **Smart Recommendation Card**: WHEA error mitigation proposals featuring a **`⏩ Coarse (+4)`** vs. **`🎯 Fine (+2)`** precision toggle.
  - **Core Table**: Displays physical cores with CPPC ranking badges (🥇 Gold & 🥈 Silver preferred cores) and live Curve Optimizer margin spinners (-60 to +30).
- **Right Column (Test Controls & Auto-Tuner)**:
  - **Profile Selector**: Presets for stress testing (e.g., *Full Test - SSE & AVX2*).
  - **🤖 Auto-Tuner Card**: Selection box for automated step-down (Disabled / Coarse / Fine).
  - **Primary Action Buttons**: `⚡ Apply CO Values Live` and `🚀 Start Test`.
  - **⚙️ Advanced Settings**: Scrollable panel for runtime per core, passes, Prime95 vs. y-cruncher selection, post-test actions (Sleep / Shutdown), safety temp limits, and Windows autostart configuration.
- **Bottom Workspace Card**:
  - Tab 1: **`📊 Live Telemetry Graph`** (60-second rolling chart of Clock MHz, Temp °C, and Power W).
  - Tab 2: **`📜 Console & Log Protocol`** (Detailed developer execution logs).

---

## 3.1 Test Profiles & Tuning Strategy (Full Run vs. Overnight)

PboStudio includes pre-configured test profiles tailored for Curve Optimizer validation:

- **⚡ Full Run (SSE & AVX2 Combined)**:
  - **Duration**: ~1.5 to 4 hours (on 8-core CPUs).
  - **Phases**: Phase 1 (SSE with Huge FFTs) + Phase 2 (AVX2 with Smallest FFTs).
  - **Purpose**: **Primary Tuning Preset**. Combines low-voltage high-boost limit testing (SSE) with heavy heat & power testing (AVX2). Perfect for standard tuning and Auto-Tuner optimization.

- **🌙 Overnight Thorough Validation**:
  - **Duration**: ~8 to 16+ hours.
  - **Phases**: 3 phases across **ALL FFT sizes** (4K to 8960K+) for SSE, AVX, and AVX2.
  - **Purpose**: **Final Rock-Solid Verification**. Tests every single frequency step, cache block, and execution unit across your CPU. Passes here guarantee 100% 24/7 stability before permanent BIOS entry.

### 💡 Recommended Tuning Workflow:
1. Run **`⚡ Full Run`** or launch **`🤖 Auto-Tuner`** to find your optimal per-core offsets quickly.
2. Once your offsets are found, run **`🌙 Overnight`** once overnight for 100% final confirmation.
3. Click **`📋 Copy BIOS List`** and enter your tested values into your motherboard BIOS.

---

## 4. System Health Audit (EXPO RAM & AGESA BIOS Check)
Clicking on **`🔍 SYSTEM: xx/100`** runs a full diagnostic audit:
1. **RAM EXPO / XMP Profile**: Queries WMI for active memory clock speed. If your RAM runs at fallback JEDEC speeds (e.g., 4800 MT/s instead of 6000 MT/s), an optimization alert is flagged.
2. **AGESA BIOS Release Date**: Audits motherboard BIOS release dates. BIOS versions older than 9 months trigger a recommendation to update for improved Ryzen stability and memory compatibility.

---

## 5. WHEA Watchdog & Smart Recommendations (Coarse vs. Fine)
The integrated Watchdog monitors the Windows Event Log in real-time for **WHEA hardware errors 18/19**, mapping them strictly in ascending numerical order to physical CPU cores (`Core 0`, `Core 1`, `Core 4`).

### Precision Modes:
- **⏩ Coarse (+4)**: Backs off voltage magnitude by `+4` steps (e.g., `-30 ➔ -26`). Recommended for quickly stabilizing volatile cores.
- **🎯 Fine (+2)**: Backs off voltage magnitude by `+2` steps (e.g., `-30 ➔ -28`). Tailored for precision tuning to preserve maximum performance.

Clicking **`⚡ Apply Values`** updates all core margins in the table instantly.

---

## 6. The Auto-Tuner Engine (Automated CO Step-Down)
The **Auto-Tuner** automatically discovers individual core stability limits:

1. **Selecting Mode**:
   - **Coarse (-3 steps)**: Fast discovery mode. Steps down passing cores by 3 points per iteration.
   - **Fine (-1 step)**: Precision mode. Steps down passing cores by 1 point per iteration.
2. **Workflow**:
   - When a core passes a test iteration, the Auto-Tuner lowers its voltage offset for the next round.
   - When a core fails or trips a WHEA error, the Auto-Tuner steps back to the last safe margin, **locks the core as optimal**, and continues testing remaining cores.

---

## 7. Applying CO Values Live vs. Motherboard BIOS Flashing

### Applying Live in Windows:
Clicking **`⚡ Apply CO Values Live`** writes offsets directly to the AMD SMU firmware.
- **Autostart Feature**: Enable `[x] Apply automatically on Windows boot` in Advanced Settings. PboStudio creates a Windows Scheduled Task to load your offsets automatically on every reboot.

### Permanent Motherboard BIOS Entry:
No Windows application can flash or alter motherboard SPI BIOS chips during runtime. To enter your tested values permanently into BIOS:
1. Note your stable core margins from PboStudio (or click **`📋 Copy BIOS List`**).
2. Reboot PC and press `DEL` or `F2` to enter BIOS.
3. Navigate to: *Advanced ➔ AMD Overclocking ➔ Precision Boost Overdrive ➔ Curve Optimizer*.
4. Set *Curve Optimizer* to **Per Core**, sign to **Negative**, and enter magnitudes (e.g., `Core 0 = 20`, `Core 1 = 25`...).
5. Press `F10` to save and exit.

---

## 8. HTML Stability Certificate Exporter
Clicking **`📄 Export Report`** generates a standalone, interactive HTML stability certificate featuring:
- Complete hardware summary (CPU, RAM, System Audit Score)
- Completed iterations, test durations, and failure counts
- Final recommended Curve Optimizer core matrix ready for printing or forum sharing.

---

## 9. Crash Protection & BSOD Recovery
If a system crash (BSOD) occurs during heavy testing:
- PboStudio saves test state after each iteration to `runs/state.json`.
- Upon reboot, PboStudio detects the interrupted state and prompts: **`✓ Apply Recommended Offset & Resume`**.

---

## 10. Frequently Asked Questions (FAQ)

**Q: Why does PboStudio require Administrator rights?**  
*A: AMD SMU firmware communication requires Ring-0 kernel access. Administrator rights are required to interact with the PawnIO driver.*

**Q: Does PboStudio work on Intel CPUs?**  
*A: Per-core stress testing (Prime95 / y-cruncher rotator) works on all processors. AMD Curve Optimizer controls are specific to Ryzen CPUs.*

**Q: How does PboStudio differ from CoreCycler?**  
*A: CoreCycler is a PowerShell wrapper script without SMU writing capabilities. PboStudio provides direct live SMU read/write, automated auto-tuning, WHEA watchdog tracing, live telemetry graphing, and a native UI without manual `.ini` file editing.*
