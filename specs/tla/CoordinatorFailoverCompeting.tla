---- MODULE CoordinatorFailoverCompeting ----
(*
 * Safety companion for CoordinatorFailover: the real coordinator is still
 * reachable and may choose a competing cut while the fallback quorum is also
 * active. Quorum intersection plus the stable per-peer ready transcript must
 * still prevent two committed cuts.
 *)
EXTENDS CoordinatorFailover
=============================================================================
