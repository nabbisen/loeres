# Adversarial Conformance Suite

Schema-3 fixtures built from the geometries RFC 027 section 11.6 names as hard
(RFC 031). Run with `cargo xtask conformance --suite adversarial`. **Reported, not
enforced**: `cargo xtask check` runs the smoke suite only, and a fixture the
kernels fail here is a finding for the architect, never something to soften.

Every expected value comes from an exact rational active-set reference, never from
a kernel (see `../README.md`); each fixture's comment states the property it
stresses and the extreme value that makes it adversarial. The runner prints the
difficulty figures (outer iterations against the cap, `projection_cap_hits`,
terminal `max_constraint_violation`) beside each fixture.

| Family (id prefix) | Property | Extreme value recorded in the fixtures |
|---|---|---|
| `qp-adv-parallel-` | nearly parallel constraint normals | angles `atan(eps)` for `eps` in 0.5, 0.1, 0.05, 0.02, 0.01, 0.005, 0.004, 0.002, 0.001; **smallest angle `atan(0.001)` = 0.001 rad** |
| `qp-adv-degenerate-box-` | degenerate boxes | `lo == hi` on one coordinate, and on every coordinate |
| `qp-adv-ill-conditioned-` | ill-conditioned `Q` | condition number 10, 100, 1000 |
| `qp-adv-barely-feasible-` | small non-zero feasible volume | triangle areas down to `5e-13`; a simplex volume of `1.7e-16` |
| `qp-adv-cancelling-` | exactly cancelling infeasible geometry (RFC 027 section 0.3.4) | exactly opposite normals; normals summing to exactly zero; each with a Farkas certificate |
