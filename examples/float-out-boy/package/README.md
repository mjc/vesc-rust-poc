# Float Out Boy

Float Out Boy is an unofficial Rust port of [Refloat](https://github.com/lukash/refloat), a full-featured self-balancing skateboard package for VESC-based controllers.

The new name is intentional: this is a separate Rust implementation, and calling it Refloat would make it too easy to confuse the port with the original project, its releases, and its support channels.

## Current behavior baseline

The `0.1.0` package tracks Refloat's post-1.2.1 behavior through the pinned 1.3 beta cutoff. This includes measured-loop timing, smoothed setpoint modifiers, motor-flux torque normalization on supported firmware, distance-driven Reverse Stop, the unified remote command and fullscreen overlay, expanded realtime plotting, data recording, footpad ADC swapping, and bounded LCM handling.

These changes can make an existing tune feel different. Back up the current configuration and review the smoothing and tilt-transition settings after upgrading.

## Installation and upgrades

Back up your package configuration before upgrading, either with **Backup Configs** on the Start page or by saving the XML from **Float Out Boy Cfg**.

For a fresh board installation, complete the motor and IMU calibration before installing the package. If the package is already installed, disable it while running motor calibration and re-enable it afterward.

The [Initial Board Setup guide](https://pev.dev/t/initial-board-setup-in-vesc-tool/2190) covers the VESC Tool setup process. On firmware 6.02, set the **Low and High Tiltback voltages** on the **Specs** tab to match the battery. Newer firmware can use the package's per-cell voltage thresholds.

## Safety

**Use at your own risk.** Electric vehicles are inherently dangerous. The authors and contributors are not liable for damage or injury caused by this software. Float Out Boy is not endorsed by the VESC project.

## Lineage and credits

Float Out Boy follows the behavior and package design of Refloat while porting the native implementation to Rust. Refloat is authored by Lukáš Hrázký and builds on the original Float package by Mitch Lustig, Dado Mista, and Nico Aleman.

- [Refloat source](https://github.com/lukash/refloat)
- [Refloat releases](https://github.com/lukash/refloat/releases)
- [Refloat 1.2 release notes](https://pev.dev/t/refloat-version-1-2/2795)
- [Refloat 1.3 release notes](https://pev.dev/t/refloat-version-1-3/2995)
