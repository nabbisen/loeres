# Extended Conformance Suite

Schema-3 fixtures above the smoke corpus's `n <= 3`, `m <= 3` (RFC 031): sizes
4 to 12, up to six halfspaces, one diagonal-`Q` case and one with active box
faces. Run with `cargo xtask conformance --suite extended`. **Reported, not
enforced**: `cargo xtask check` runs the smoke suite only. Every expected value is
derived from an exact active-set reference, never from a kernel; see
`../README.md`.
