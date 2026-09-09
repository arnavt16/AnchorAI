# Security & privacy

What Anchor actually protects against, what it doesn't, and where the
honest limits are. Corresponds to the "Desktop privacy and security"
section of `docs/spec.md`.

## Threat model

What the MVP protects against:

**Accidental cloud disclosure.** Journal content, prompts, embeddings,
and reflection responses never leave your computer. The Ollama HTTP
client (`src-tauri/src/ollama/mod.rs`) refuses to build a request to
anything other than `127.0.0.1` / `localhost`, disables redirects, and
ignores system proxy settings, even if a config file or dev-time env var
is tampered with.

**Renderer overreach.** The webview (the React frontend) has no shell
execution, no arbitrary filesystem access, and no generic HTTP proxy. It
can only call the narrow `#[tauri::command]` functions listed in
`src-tauri/src/lib.rs`'s `generate_handler!` and scoped by
`src-tauri/capabilities/default.json`. Every command re-validates its
own input in Rust — the frontend's Zod checks (`src/lib/schemas.ts`) are
a UX convenience, not a trust boundary.

**SQL injection.** Everything goes through `rusqlite` parameter binding
(`src-tauri/src/db/repo.rs`). No string-concatenated SQL anywhere.

**Prompt injection from journal content.** Retrieved journal text is
passed to the local chat model as clearly-delimited untrusted data, with
an explicit system instruction not to treat it as commands (see
`src-tauri/src/rag/generation.rs`). `evals/fixtures/demo_entries.json`
has an entry (`e16`) that's specifically a prompt-injection attempt, to
test this. It's a prompt-level mitigation, not a guarantee — see
"Limitations" below. Same framing applies on the natural-language
fallback path (`PLAIN_SYSTEM_INSTRUCTIONS`), it just isn't paired with a
JSON citation array.

What the MVP does **not** protect against:

**An unlocked computer, malware, or a machine administrator.** The vault
is a plain SQLite database in your OS user's application-data directory,
protected only by normal OS file permissions, not app-level encryption.
If your disk isn't encrypted (FileVault / BitLocker / LUKS) and someone
gets physical or admin access, they can read the vault file directly with
any SQLite tool. Anchor doesn't implement database encryption — that's
out of scope for this milestone and would need a real key-management
design, not a password gate bolted on.

**A compromised or malicious Ollama installation**, or any other local
process that can reach `127.0.0.1:11434`. Loopback-only communication
cuts down network exposure but isn't authentication — anyone with code
execution on your machine could talk to the same Ollama instance.

**Forensic recovery.** Deleting an entry or erasing the vault removes it
from the live SQLite file but doesn't securely wipe the underlying
storage — SSD wear-leveling, the SQLite WAL file's transient state,
filesystem journals, OS snapshots (Time Machine, etc.), and any backups
you made yourself can all still hold copies. "Erase local data" in
Settings says this plainly in its confirmation text.

**A remote/cloud-backed Ollama model pretending to be local.** Ollama can
be configured to proxy some model names to a cloud service. Anchor's
client only accepts a loopback host, but it can't verify from the client
side whether Ollama's own backend for a given model is actually local
hardware or a cloud relay. See Ollama's docs on cloud-disabled mode
(linked from Setup) and check your own Ollama config if this matters to
you.

## Data at rest

- Vault location: your OS's per-user application data directory (shown
  in Settings), e.g. `~/Library/Application Support/com.anchor.journal/vault/`
  on macOS. Not inside the app bundle, not in Downloads, not synced
  anywhere by default.
- Format: plain (not app-encrypted) SQLite, WAL mode. Your OS disk
  encryption, if you have it on, is what protects this at rest — Anchor
  doesn't add its own layer.
- Demo vault: a separate SQLite file (`demo-vault/anchor-demo.sqlite`),
  never merged with your personal vault. Switching between them bumps an
  internal `vault_generation` counter so in-flight background work from
  one can never commit into the other.

## What never leaves your computer

Journal text, worry/outcome/step text, embeddings, prompts sent to the
model, and model responses. The only network call Anchor's own code
makes is to `127.0.0.1:11434`. No telemetry, no crash reporting, no
analytics, no remote fonts, no update-check network call anywhere in
this codebase.

## Logging

`src-tauri/src/error.rs` defines a small set of `SafeErrorCode` values
(`database_locked`, `model_missing`, etc.) — that's the only thing that
reaches both the UI and the local `tracing` log on error paths. Raw
`rusqlite` error text (which can occasionally echo back bound values) is
deliberately kept out of logs and the UI — see the
`From<rusqlite::Error> for AnchorError` impl. No command handler logs
entry bodies, worry text, or model prompts/responses.

## The urgent-distress path is not clinical triage

`src-tauri/src/safety/mod.rs` is a conservative, local, keyword-anchored
classifier that routes evident imminent-danger statements to a bundled
supportive response instead of normal reflection. Read that file's
module doc before relying on it for anything. Short version:

- It's biased toward **false positives** (flagging something merely
  dramatic) over **false negatives** (missing a real disclosure) — the
  cost of the former is low, the cost of the latter isn't.
- It's a substring/phrase matcher, not a language model or a validated
  clinical instrument. `evals/fixtures/safety_fixtures.json` documents
  known cases where it gives the "safe" wrong answer (fixture `s7`).
- The bundled crisis-resource list (`safety::CRISIS_RESOURCES`) currently
  has exactly two entries (US, UK), both marked
  `verified_at: "unverified-in-this-build"` — meaning I haven't
  re-checked them against live sources for this build. That needs to
  happen before any real use.
- Human review of model behavior on the safety fixtures — and of this
  whole subsystem — is required before any public or clinical-adjacent
  release. The automated tests (`src-tauri/tests/safety_tests.rs`) only
  exercise the deterministic keyword layer; they say nothing about how
  the actual chat model handles ambiguous or figurative distress
  language in a live conversation. That requires running the fixtures
  through a real model and reading the transcripts by hand, which I
  haven't done yet.

## Local runtime limitations

Ollama is separate, shared software — Anchor doesn't bundle it, doesn't
manage its lifecycle beyond talking to its HTTP API, and can't guarantee
what else might be talking to the same Ollama instance, what request
logging Ollama does, or what it keeps in memory or on disk between
requests. Anchor doesn't touch your global Ollama config.

## Reporting a concern

This is a portfolio project, so there's no formal disclosure process. If
you're evaluating this codebase and find something, the most useful
thing is a specific file/line and a concrete scenario — this doc is meant
to make it easy to tell "not implemented yet, documented above" apart
from "actually a bug."
