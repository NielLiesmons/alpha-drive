# Alpha Drive

Peergos-shaped encrypted Drive: **server stores ciphertext only**; **Space root capability stays on devices**.

v0 = **one Space root key** on every member device (fine for full-trust groups). Fine-grained cryptree caps later.

## Status

- Content-addressed **ciphertext** block store (`PUT/GET /v1/blocks/{sha256}`)
- Encrypted **tip** → encrypted `alpha.dir/1` index
- CLI: `keygen`, `serve`, `init`, `ls`, `put`, `get`, `project`, `task`, `note`
- Schemas: `schemas/alpha.project.1.json`, `alpha.task.2.json` (+ `/1` legacy), `alpha.note.1.json`, `alpha.media_blob.1.json`, `alpha.track.1.json`, `alpha.playlist.1.json`, `alpha.dir.1.json`, stubs `alpha.ai_thread.1.json` / `alpha.git_repo.1.json`
- Content-type packages: `content-types/` (manifest + declarative chat card + open-with) — see `../alpha-docs/ADR-content-type-packages.md`
- Chunked **Player** media (`media::put_media_blob` / `get_media_chunk`) — stream-on-play before mailboxes; see `../alpha-docs/PLAYER.md`
- Live: `https://drive.example.com/health` → `ok` (data on Storage Box)

## Keys (v0 family)

| Item | Who holds it | Purpose |
|------|----------------|---------|
| **Space root capability** | Phones / laptops + offline recovery kit | Encrypt / decrypt everything |
| **API bearer token** | Apps + server env | Authn to store/fetch ciphertext — **not** crypto |

Same root key on every Space member’s devices = full access for all of them. Revoke later via cryptree shares; for now, rotate the Space key only if you must.

```bash
cargo run -- keygen   # print once → password manager + recovery paper
export ALPHA_DRIVE_ROOT_KEY=…   # devices only
export ALPHA_DRIVE_TOKEN=…      # also in /etc/alpha-drive/env on the VPS
```

## Tree layout

Logical paths (names inside the encrypted root dir):

```
meta/space.json
projects/{id}/project.json     # alpha.project/1
projects/{id}/home/…           # optional narrative folder
tasks/{id}/task.json           # alpha.task/2  (project_ids[])
notes/{id}/note.json           # alpha.note/1  (project_ids[])
files/…  media/…               # content-type blobs (+ labels later)
```

Projects are **entities + labels**, not exclusive folders. Tasks and Notes live under their own content-type roots and link via `project_ids[]`. `home/` is just the project’s description folder. Blobs drop into `files/` / `media/`; query indexes are separate. See `../alpha-docs/DRIVE-LAYOUT.md`.

## Quick start (local)

```bash
cd alpha-drive
cargo build

export ALPHA_DRIVE_ROOT_KEY="$(cargo run --quiet -- keygen)"
export ALPHA_DRIVE_TOKEN=dev-token

cargo run -- serve --data ./data --token "$ALPHA_DRIVE_TOKEN" &

cargo run -- init --name "Demo Space"
cargo run -- project create --title "Demo Space" --slug demo-space
# → prints JSON including id
cargo run -- task create --project <id> --project <other-id> --title "Order CSA boxes"
cargo run -- note create --project <id> --title "Drop-off planning" --body "## This week"
cargo run -- ls
cargo run -- project list
cargo run -- task list --project <id>
cargo run -- note list --project <id>
```

Against production (root key still only on your laptop):

```bash
export ALPHA_DRIVE_TOKEN=…   # from the VPS env — never commit
cargo run -- project create --url https://drive.example.com --title "…"
```

## API (authority)

| Method | Path | Body |
|--------|------|------|
| GET | `/health` | |
| PUT | `/v1/blocks/{hash}` | ciphertext bytes; `hash` = SHA-256(ciphertext) |
| GET | `/v1/blocks/{hash}` | ciphertext |
| PUT | `/v1/tip` | `{"root_hash":"<block hash>"}` |
| GET | `/v1/tip` | tip JSON |
| PUT | `/v1/tips/{name}` | named tip (e.g. `chat`) — does not move Space tip |
| GET | `/v1/tips/{name}` | named tip JSON |
| PUT | `/v1/mailboxes/{name}/append` | one-RTT append: body = ciphertext; headers `x-alpha-hash`, `x-alpha-prev` (CAS). 409 + current tip on conflict |

All `/v1/*` require `Authorization: Bearer <token>`.

## Tests

```bash
cargo test
```

Includes Project/Task/Note round-trip, chunked media round-trip, and “wrong root key cannot read.”

## Toward full cryptree

- Dir entries already carry `wrapped_key` (file key inside the encrypted dir node).
- Chunked media blobs: CAS chunks + `alpha.media_blob/1` manifest (Player stream-on-play).
- Next: Admin Player upload/play UI, nested `alpha.dir` trees, per-person subtree caps.

See `../alpha-docs/MVP-family-drive-projects-tasks.md`.
