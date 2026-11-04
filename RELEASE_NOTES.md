# Lienzo 0.1.9

This release fixes a Windows freeze when moving the Lienzo window between
monitors with different display scale factors.

## Fixes

- During a live window drag, Lienzo now applies the DPI-adjusted rectangle
  suggested by Windows. This prevents repeated scale changes and runaway
  resizing when crossing mixed-DPI monitor boundaries.
- The existing DPI adjustment remains in place outside a live drag.
- The fix was confirmed by dragging Lienzo between monitors with different
  scaling on Windows.

## Packages

GitHub Actions validated the code and published six packages for Windows
x86_64, macOS ARM64, and Linux x86_64, along with tag `v0.1.9` and
`SHA256SUMS.txt`. The binaries are not commercially signed, so macOS Gatekeeper
or Windows SmartScreen may show a warning. Use `SHA256SUMS.txt` to verify each
download.
