# Alpha Drive

Peergos-shaped encrypted Drive: **server stores ciphertext only**; **family root capability stays on devices**.

v0 = **one family key** on every device (fine for full household access). Fine-grained cryptree caps later.

## Status

- Content-addressed **ciphertext** block store (`PUT/GET /v1/blocks/{sha256}`)
- Encrypted **tip** → encrypted `alpha.dir/1` index
- CLI: `keygen`, `serve`, `init`, `ls`, `put`, `get`, `project`, `task`
- Schemas: `schemas/alpha.project.1.json`, `alpha.task.2.json` (+ `/1` legacy), `alpha.dir.1.json`
- Live: `https://drive.bley.farm/health` → `ok` (data on Storage Box)

## Keys (v0 family)

| Item | Who holds it | Purpose |
|------|----------------|---------|
| **Family root capability** | Phones / laptops + offline recovery kit | Encrypt / decrypt everything |
| **API bearer token** | Apps + server env | Authn to store/fetch ciphertext — **not** crypto |

Same root key on Niel’s and partner’s devices = full access for both. Revoke later via cryptree shares; for now, rotate the family key only if you must.

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
files/…  media/…               # content-type blobs (+ labels later)
```

Projects are **entities + labels**, not exclusive folders. A file can carry many `project_ids`; `home/` is just the project’s description folder. See `../alpha-docs/DRIVE-LAYOUT.md`.

## Quick start (local)

```bash
cd alpha-drive
cargo build

export ALPHA_DRIVE_ROOT_KEY="$(cargo run --quiet -- keygen)"
export ALPHA_DRIVE_TOKEN=dev-token

cargo run -- serve --data ./data --token "$ALPHA_DRIVE_TOKEN" &

cargo run -- init --name "Bley Family"
cargo run -- project create --title "Ferme de Bley" --slug ferme-de-bley
# → prints JSON including id
cargo run -- task create --project <id> --project <other-id> --title "Order CSA boxes"
cargo run -- ls
cargo run -- project list
cargo run -- task list --project <id>
```

Against production (root key still only on your laptop):

```bash
export ALPHA_DRIVE_TOKEN=…   # from the VPS env — never commit
cargo run -- project create --url https://drive.bley.farm --title "…"
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

## Tests

```bash
cargo test
```

Includes Project/Task round-trip and “wrong root key cannot read.”

## Toward full cryptree

- Dir entries already carry `wrapped_key` (file key inside the encrypted dir node).
- Next: nested `alpha.dir` trees, per-person subtree caps, encrypted music index + stream-on-play, Admin wiring.

See `../alpha-docs/MVP-family-drive-projects-tasks.md`.
