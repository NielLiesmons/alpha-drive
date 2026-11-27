# Alpha Drive

Encrypted ciphertext store for Alpha Spaces. Cryptree model based on the [Peergos](https://peergos.org/) cryptree (Wuala paper + Peergos caps): **server stores ciphertext only**; Space root capability stays on devices.

Alpha = private Drive + Mail + Chat on your domain — open protocols, self-hostable where it matters.

## Model

| Piece | Who holds it | Role |
|-------|----------------|------|
| Space root key | Member devices (+ recovery kit) | Encrypt / decrypt everything |
| API bearer token | Apps + server env | Authn to CAS — **not** crypto |
| Tip | Named pointers (`root`, `chat`, …) | Current encrypted dir / mailbox head |
| Blocks | SHA-256 addressed ciphertext | Content-addressed store |

v0 = one Space root on every member device (full-trust group). Fine-grained cryptree caps later.

## Stack

- Rust binary: `keygen`, `serve`, `init`, `ls`, `put`, `get`, `project`, `task`, `note`
- Schemas: `schemas/alpha.*.json` (+ `content-types/` packages)
- Chunked media: `alpha.media_blob/1` for Player stream-on-play

## Tree (logical paths inside encrypted root)

```
meta/space.json
projects/{id}/project.json
tasks/{id}/task.json      # project_ids[]
notes/{id}/note.json
files/…  media/…
```

Projects are **labels**, not exclusive folders. See sibling `alpha-docs/DRIVE-LAYOUT.md`.

## API

All `/v1/*` need `Authorization: Bearer <token>`.

| Method | Path | Notes |
|--------|------|-------|
| GET | `/health` | |
| PUT/GET | `/v1/blocks/{sha256}` | Ciphertext CAS |
| PUT/GET | `/v1/tip` | Space tip |
| PUT/GET | `/v1/tips/{name}` | Named tip (e.g. `chat`) |
| PUT | `/v1/mailboxes/{name}/append` | One-RTT append; `x-alpha-hash` + `x-alpha-prev` CAS |

## Local

```bash
cargo build
export ALPHA_DRIVE_ROOT_KEY="$(cargo run --quiet -- keygen)"
export ALPHA_DRIVE_TOKEN=dev-token
cargo run -- serve --data ./data --token "$ALPHA_DRIVE_TOKEN" &
cargo run -- init --name "Demo Space"
```

Never commit root keys or production tokens. Deploy env lives on the VPS only.
