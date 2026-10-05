# Alpha Drive

Peergos-shaped encrypted Drive: **server stores ciphertext only**; **root capability stays on devices**.

Open-ended toward full cryptree (per-directory wrapped keys). v0 uses one/few client-held root caps for speed.

## Status

Working local MVP:

- Content-addressed **ciphertext** block store (`PUT/GET /v1/blocks/{sha256}`)
- Encrypted **tip** pointer (`GET/PUT /v1/tip`) → encrypted `alpha.dir/1` index
- CLI: `keygen`, `serve`, `put`, `get`
- Schemas: `schemas/alpha.project.1.json`, `alpha.task.1.json`, `alpha.dir.1.json`

## Threat model

| On server | On devices |
|-----------|------------|
| Opaque blocks + tip hash | Root capability (hex key) |
| Bearer token (authn only) | Encrypt before upload / decrypt after download |

Losing all devices **and** the recovery copy of the root key = data loss.

## Quick start

```bash
cd alpha-drive
cargo build

# 1) Root capability — keep offline / on phones & laptops only
cargo run -- keygen   # → print hex; export ALPHA_DRIVE_ROOT_KEY=...

# 2) API token — not crypto; only gates who may store/fetch blocks
export ALPHA_DRIVE_TOKEN=dev-token

# 3) Authority (no root key here)
cargo run -- serve --data ./data --token "$ALPHA_DRIVE_TOKEN"

# 4) Client (other terminal)
echo 'CSA opens Monday' > /tmp/note.txt
cargo run -- put --token "$ALPHA_DRIVE_TOKEN" --root-key "$ALPHA_DRIVE_ROOT_KEY" \
  files/note.txt /tmp/note.txt
cargo run -- get --token "$ALPHA_DRIVE_TOKEN" --root-key "$ALPHA_DRIVE_ROOT_KEY" \
  files/note.txt
```

Put a Project record the same way (client encrypts JSON that matches `alpha.project/1`):

```bash
# write project.json locally, then:
cargo run -- put ... projects/<id>/project.json ./project.json --kind project
```

## API (authority)

| Method | Path | Body |
|--------|------|------|
| GET | `/health` | |
| PUT | `/v1/blocks/{hash}` | ciphertext bytes; `hash` = SHA-256(ciphertext) |
| GET | `/v1/blocks/{hash}` | ciphertext |
| PUT | `/v1/tip` | `{"root_hash":"<block hash>"}` |
| GET | `/v1/tip` | tip JSON |

All `/v1/*` require `Authorization: Bearer <token>`.

## Toward full cryptree

- Dir entries already have optional `wrapped_key` (file key stored inside the **encrypted** dir node today).
- Next: nested `alpha.dir` trees, wrap child keys to parent dir keys, public subtree read-cap, Milestone schema, Alpha Chat client.

See `../alpha-docs/MVP-family-drive-projects-tasks.md`.
