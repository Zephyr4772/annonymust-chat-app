# Anonymous Mesh Chat

**A desktop messenger designed for anonymity: onion-routed messages, end-to-end
crypto, sharded storage, and an offline mesh fallback.** Rust + Tauri 2.

> ## ⚠️ This repository is incomplete and does not build.
>
> The Tauri desktop shell is here. The five crates that do the actual work are
> **not committed** — `Cargo.toml` references them by relative path
> (`messaging`, `core_crypto`, `sharding`, `onion_routing`, `offline_mesh` at
> `../../`), and those directories are outside this repository.
>
> `cargo build` will fail on unresolved path dependencies. This is published as a
> design and work-in-progress reference, not something you can run today.

---

## The idea

Most "anonymous" chat apps are anonymous only to other users — the server still
sees who talks to whom. The design here removes that:

- **Onion routing** — messages traverse a multi-hop circuit, so no single relay knows both ends. `circuit_manager.rs` builds and maintains circuits.
- **Sharding** — message payloads are split across shards rather than stored whole.
- **Offline mesh** — peers reachable on a local network exchange messages without any internet path (`simple_mesh.rs`, `mesh_protocol.rs`, `offline_mesh`).
- **Local-only storage** — SQLite via `rusqlite`, on-device.
- **BLAKE3** for hashing, with crypto isolated in a `core_crypto` crate.

## What's in this repo

```
desktop/
  src/                      Svelte front-end
  src-tauri/
    src/
      lib.rs                Tauri app setup, command registration
      circuit_manager.rs    onion circuit construction / lifecycle
      message_router.rs     routing decisions across circuits + mesh
      group_manager.rs      group membership and keys
      file_transfer.rs      chunked direct file transfer
      mesh_protocol.rs      mesh wire format
      mesh_commands.rs      Tauri commands for mesh control
      mesh_init.rs          mesh bring-up
      simple_mesh.rs        local-network peer discovery
    Cargo.toml              ← references the 5 missing crates
```

## Stack

Rust 2021 · Tauri 2 · Svelte · tokio · rusqlite (bundled SQLite) · blake3 ·
bincode · serde · anyhow / thiserror · uuid · chrono

## To make this buildable

The missing crates need to be committed as a Cargo workspace:

```
Cargo.toml              [workspace] members = [...]
core_crypto/
messaging/
sharding/
onion_routing/
offline_mesh/
desktop/
```

Then `desktop/src-tauri/Cargo.toml` paths resolve and `cargo tauri dev` works.

## Security notice

**Do not use this for anything that matters.** The cryptographic and routing
design has had no external review, no threat model is published, and the
implementation is unfinished. Anonymity systems fail in subtle ways —
treat this as a learning project, because that's what it is.

## License

Not yet licensed — all rights reserved.
