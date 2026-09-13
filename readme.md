<!--
SPDX-FileCopyrightText: 2026 Yifei Sun
SPDX-License-Identifier: Apache-2.0
-->

# Drac

Drac is a single configuration plane for a global anycast fleet. One daemon owns
a node's network state and converges it toward a versioned, content addressed
generation. Moving between generations is confirmed or automatically reverted,
following the model of NixOS generations and JunOS confirmed commits.

The name comes from the [Drac](https://en.wikipedia.org/wiki/Drac_(river)), a
river in the French Alps.

## Status

Early, and deployed nowhere. Two crates exist, `drac-cli` and `drac-config`.
Stage 0 is in progress and covers the local generation machinery together with
an authoritative DNS server on a single node, with no network control plane yet.

## Scope

Planned subsystems, in build order:

1. Generation machinery: TOML and JSON ingest, canonical encoding, content
   addressed identifiers, an apply journal and a confirm timer.
2. Authoritative DNS served from the active generation.
3. A replicated control plane over Raft, with a small set of voting nodes and
   log replication to the rest.
4. Routing configuration, driving BIRD for BGP.
5. An IPsec mesh with its own keying and an embedded Babel speaker.
6. A caching proxy with a web application firewall.
7. Native clients for iOS, Android and desktop.

Two properties hold throughout. Reconciliation is level based: a subsystem
converges toward its target from any observed state, including after a crash
partway through an apply. Consensus covers desired configuration alone, and
runtime state such as routing tables, neighbors and cache contents converges
through the protocols that own it.

## Build and Test

The Nix development shell carries the toolchain, and direnv loads it from
`.envrc`.

```sh
cargo nextest run                                # tests
nix fmt                                          # clippy, rustfmt, deno fmt, nixpkgs-fmt, taplo
nix build .#legacyPackages.<system>.crates.<crate>
cargo kani -p <crate>                            # inline proof harnesses
```

Crates under `crates/` are discovered by Nix and by CI without further
configuration. External dependencies are pinned once in
`[workspace.dependencies]` at the repository root, because every crate links
into one binary and version skew across them is never useful.

## License

Apache-2.0, in [license.txt](license.txt). Every file carries an SPDX header,
and the generated files that cannot are declared in [REUSE.toml](REUSE.toml).
