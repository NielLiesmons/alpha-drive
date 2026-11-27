# Frame bridge (`alpha.frame/1`)

Host API for content-type packages with `"open": { "mode": "frame" }`.  
Inspired by [webxdc](https://webxdc.org/) status updates; **Drive entity + schema remain source of truth**.

Team implementation: `alpha-team/src/lib/content-types/frame-host.js` + `ContentTypeFrame.svelte`.

## Non-negotiables

- Package UI runs **only** inside the frame (or a first-party native route).
- Chat feed never loads package JS/CSS. Cards stay declarative (`chat.json`).
- Quiet activity in chat uses short `info` / `summary` strings the host renders with the package icon + type name.

## JS surface (`window.alphaFrame`)

Injected by the host into the sandboxed document:

| Call | Purpose |
|------|---------|
| `getIdentity()` | `{ email, name }` for the signed-in member |
| `getEntity()` | Current Drive leaf (validated against package `entity_schema`) |
| `putEntity(patch)` | Merge/replace leaf; host validates schema then writes Drive |
| `sendActivity({ kind, info?, summary? })` | Quiet timeline line / card summary (webxdc-like) |
| `close()` | Dismiss frame |

Transport: `postMessage` with `{ bridge: "alpha.frame/1", id, method, params }` → reply `{ bridge, id, result | error }`.

Sandbox: `allow-scripts allow-forms allow-downloads allow-modals` — **not** `allow-same-origin` on blob frames.

## Chat mapping

| Frame activity | Feed |
|----------------|------|
| `info` | Quiet activity line on the object card |
| `summary` | Replaces card summary next to type name |
| (entity put) | May also emit `activity.update` from `chat.json` templates |

Do not spam `info` on every keystroke; aggregate like webxdc.

## Install path

Third-party packages: `meta/content_types/{package_id}/` on the Space Drive  
(`manifest.json`, `chat.json`, optional `app/index.html`). See `install.js` in Admin.
