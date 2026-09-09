# Security & privacy

This document describes what Anchor actually protects against, what it
doesn't, and where the honest limits are. It corresponds to the "Desktop
privacy and security" and architecture/privacy sections of `docs/spec.md`.

## Threat model

Anchor's MVP protects against:

- **Accidental cloud disclosure.** Journal content, prompts, embeddings, and
  reflection responses never leave your computer. The Ollama HTTP client
  (`src-tauri/src/ollama/mod.rs`) refuses to construct a request to any host
  other than `127.0.0.1` / `localhost`, disables HTTP redirects, and ignores
  system proxy settings — even if a setting file or dev-time environment
  variable is tampered with to point somewhere else.
- **Renderer overreach.** The webview (the React frontend) has no shell
  execution, no arbitrary filesystem access, and no generic HTTP proxy. It
  can only call the narrow, individually-validated `#[tauri::command]`
  functions listed in `src-tauri/src/lib.rs`'s `generate_handler!` call and
  scoped by `src-tauri/capabilities/default.json`. Every command
  re-validates its own input in Rust — the frontend's Zod checks
  (`src/lib/schemas.ts`) are a UX convenience, not a trust boundary.
- **SQL injection.** All queries use `rusqlite` parameter binding
  (`src-tauri/src/db/repo.rs`); there is no string-concatenated SQL
  anywhere in this codebase.
- **Prompt injection from journal content.** Retrieved journal text is
  passed to the local chat model as clearly-delimited untrusted data, with
  an explicit system instruction not to treat it as commands (see
  `src-tauri/src/rag/generation.rs`). `evals/fixtures/demo_entries.json`
  includes an entry (`e16`) designed to test this. This is a prompt-level
  mitigation, not a guarantee — see "Limitations" below. This applies
  equally on the natural-language fallback path described next — the
  untrusted-data framing is identical in `PLAIN_SYSTEM_INSTRUCTIONS`, it
  just isn't paired with a JSON citation array.

Anchor's MVP does **not** protect against:

- **An unlocked computer, malware, or a machine administrator.** The vault
  is a plain SQLite database in your OS user's application-data directory,
  protected only by normal OS file permissions — not application-level
  encryption. If your disk isn't encrypted (FileVault on macOS, BitLocker
  on Windows, LUKS on Linux) and someone gets physical or admin access to
  the machine, they can read the vault file directly with any SQLite tool.
  **Anchor does not currently implement database encryption.** Adding it is
  explicitly out of scope for this milestone and would need a real
  key-management design, not an ad-hoc password gate.
- **A compromised or malicious Ollama installation**, or another local
  process capable of reaching `127.0.0.1:11434`. Loopback-only
  communication reduces exposure to the network but is not authentication.
  Anyone with code execution on your machine could, in principle, talk to
  the same Ollama instance Anchor uses.
- **Forensic recovery.** Deleting an entry or erasing the vault removes it
  from the live SQLite file, but does not securely wipe the underlying
  storage — SSD wear-leveling, the SQLite WAL file's transient state,
  filesystem journals, OS-level snapshots (e.g. Time Machine), and any
  backups you made yourself can all retain copies. "Erase local data" in
  Settings is honest about this in its confirmation text.
- **A remote/cloud-backed Ollama model masquerading as local.** Ollama
  itself can be configured to proxy some model names to a cloud service.
  Anchor's client only accepts a loopback host, but it cannot fully verify,
  from the client side, whether Ollama's own backend for a given model name
  is actually local hardware versus a cloud relay Ollama itself is
  configured to use. See Ollama's own documentation on cloud-disabled mode,
  linked from Setup, and verify your Ollama configuration directly if this
  matters to you.

## Data at rest

- Vault location: your OS's per-user application data directory (shown in
  Settings), e.g. `~/Library/Application Support/com.anchor.journal/vault/`
  on macOS. Not inside the installed app bundle, not in Downloads, not in a
  sync folder by default.
- Format: plain (not application-encrypted) SQLite, WAL mode. Your OS disk
  encryption, if enabled, is what protects this at rest — Anchor does not
  add its own layer.
- Demo vault: a *separate* SQLite file (`demo-vault/anchor-demo.sqlite`),
  never merged with your personal vault. Switching between them bumps an
  internal `vault_generation` counter so in-flight background work from one
  can never commit into the other.

## What never leaves your computer

Journal text, worry/outcome/step text, embeddings, prompts sent to the
model, and model responses. The only network traffic Anchor's own code
initiates is to `127.0.0.1:11434` (your local Ollama). There is no
telemetry, crash reporting, analytics, remote font loading, or
update-check network call anywhere in this codebase.

## Logging

`src-tauri/src/error.rs` defines a small set of `SafeErrorCode` values
(e.g. `database_locked`, `model_missing`) that are the *only* thing surfaced
to both the UI and the local `tracing` log output on error paths. Raw
`rusqlite` error text — which can in rare cases echo back bound values — is
deliberately **not** included in what gets logged or shown; see the
`From<rusqlite::Error> for AnchorError` implementation. No command handler
logs entry bodies, worry text, or model prompts/responses.

## The urgent-distress path is not clinical triage

`src-tauri/src/safety/mod.rs` implements a conservative, local,
keyword-anchored classifier that routes evident imminent-danger statements
to a bundled supportive response instead of normal reflection. Read that
file's module doc in full before relying on it for anything. In short:

- It is biased toward **false positives** (flagging something merely
  dramatic) over **false negatives** (missing a real disclosure), on the
  theory that the cost of the former is low and the cost of the latter is
  high.
- It is a substring/phrase matcher, not a language model or a validated
  clinical instrument. `evals/fixtures/safety_fixtures.json` documents
  known cases where this produces the "safe" wrong answer (see fixture
  `s7`).
- The bundled crisis-resource list in `safety::CRISIS_RESOURCES` currently
  contains exactly two entries (US, UK) and both are marked
  `verified_at: "unverified-in-this-build"` — meaning this build has not
  re-checked them against their live sources. **Before any real use, a
  human must review and re-verify this list.**
- **Human review of model behavior on the safety fixtures, and of this
  entire subsystem, is required before any public or clinical-adjacent
  release.** The automated tests (`src-tauri/tests/safety_tests.rs`)
  exercise the deterministic keyword layer only — they say nothing about
  how the actual chat model behaves on ambiguous or figurative distress
  language in a live conversation. That requires running the fixtures
  through a real model and reading the transcripts by hand.

## Local runtime limitations

Ollama is separate, shared software — Anchor does not bundle it, does not
manage its lifecycle beyond talking to its HTTP API, and cannot guarantee
what other local tools might also be talking to the same Ollama instance,
what request logging Ollama itself performs, or what temporary state it
retains in memory or on disk between requests. Anchor does not silently
change your global Ollama configuration.

## Reporting a concern

This is a portfolio project without a formal disclosure process. If you're
evaluating this codebase and find a security issue, the most useful thing
is a specific file/line and a description of the concrete scenario where it
matters — this document is meant to make it easy to tell the difference
between "not yet implemented, documented above" and "actually a bug."
