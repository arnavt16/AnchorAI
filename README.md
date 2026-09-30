# Anchor

[![CI](https://github.com/arnavt16/AnchorAI/actions/workflows/ci.yml/badge.svg)](https://github.com/arnavt16/AnchorAI/actions/workflows/ci.yml)

Anchor is a local-first desktop journal. The feature I actually built this
project around is **Worry Loop**: it connects a worry you're having right
now to relevant past worries, the outcomes you recorded for them, and the
things you said helped — retrieved and generated entirely on your own
computer, no cloud involved.

I built Anchor as an applied LLM/RAG portfolio project. It's not a
clinical product, isn't clinically validated, doesn't diagnose, and isn't
a substitute for professional care. [SECURITY.md](./SECURITY.md) has the
full privacy/threat-model writeup, and [Project status](#project-status)
below says exactly what's implemented, tested, and still missing.

<!--
  Screenshot(s) go here. Once you've run `npm run tauri dev` locally:
  1. Take a screenshot of the Reflect screen (ideally showing a cited
     response with the source drawer open) and/or the Worry Loop screen.
  2. Save it as docs/screenshots/reflect.png (create the folder).
  3. Replace this comment block with:

     ![Anchor's Reflect screen, showing a memory-grounded response with cited sources](./docs/screenshots/reflect.png)
-->

## Why I built this

I wanted a portfolio project that was more than a wrapper around an API
call — something with a real local RAG pipeline, a real desktop app
shell, and a feature that couldn't just be "chatbot with a system
prompt." Worry Loop is that feature: it's not enough to retrieve similar
journal entries, you also have to follow the link from a worry to its
recorded outcome, and you have to not paper over unfavorable outcomes
just because that would make for a nicer-sounding response. Getting that
right (and testing that it stays right) ended up being most of the
actual engineering work.

## What's here right now

A working desktop shell, full journal + Worry Loop CRUD, a real local
embedding/RAG pipeline against Ollama, multi-turn reflection with source
citations, a one-command retrieval eval harness, CI on Linux and macOS,
and the safety/privacy plumbing (consent, memory exclusion, two-tier
distress routing, session-only chat, Rust-owned file dialogs). Packaging a signed
installer, a proper usability study, and a few convenience features are
**not** done yet — see [Project status](#project-status).

## For end users: installing and running Anchor

You need two things: **Anchor itself**, and **Ollama**, which runs the
local models Anchor's reflection feature uses. You can also just use
Anchor as a plain journal with no Ollama at all.

### 1. Install Ollama (optional, only needed for reflection)

1. Download Ollama from [ollama.com](https://ollama.com) and install it normally.
2. Pull a chat model and an embedding model, e.g.:
   ```
   ollama pull llama3.1:8b
   ollama pull nomic-embed-text
   ```
   These are just what I used — any local chat + embedding model pair
   Ollama can run should work; smaller/larger models trade speed for
   reflection quality depending on your hardware.
3. Make sure Ollama is running (`ollama list` should show your pulled models).
4. If you want the strongest local-only guarantee, also turn on Ollama's
   cloud-disabled setting (see [Ollama's FAQ](https://docs.ollama.com/faq)).
   Anchor's own network client already refuses any non-loopback host, but
   Ollama itself can be configured to proxy to cloud models — that's a
   setting on Ollama's side, not Anchor's.

### 2. Get Anchor running

I originally developed and smoke-tested this on Linux. It now builds and
passes its full test suite on macOS 15 (Apple Silicon), and CI runs the
tests on both Ubuntu and macOS. There's no signed installer yet (see
[Project status](#project-status)). To build it yourself on a Mac:

1. Install [Node.js](https://nodejs.org) (v20+) and [Rust](https://www.rust-lang.org/tools/install) (`curl https://sh.rustup.rs -sSf | sh`).
2. Install Xcode Command Line Tools: `xcode-select --install`.
3. From this project's folder:
   ```
   npm install
   npm run tauri build
   ```
   The first build compiles Rust dependencies and takes a few minutes. The
   app bundle lands under `src-tauri/target/release/bundle/macos/` (and a
   `.dmg` under `bundle/dmg/` if that target succeeds on your machine).
4. Open `Anchor.app`. macOS Gatekeeper will probably warn that it's from
   an unidentified developer (it's not signed or notarized yet). Right-click
   the app and choose "Open" once to get past this, or allow it in
   System Settings → Privacy & Security.
5. To iterate without a full release build, use `npm run tauri dev` instead of step 3.

Your journal lives in your Mac's per-user Application Support directory
(shown in Settings once the app is running), not inside the app bundle —
deleting or reinstalling the app won't touch it.

### If something's not working

- **"Local AI unavailable" in Setup**: check `ollama list` shows your models, and that nothing else is on port 11434.
- **App won't open on macOS**: almost always Gatekeeper — see step 4 above.
- **Reflection is slow or times out**: try a smaller chat model; this depends a lot on your Mac's RAM and whether it's Apple Silicon.
- The journal itself (create/edit/search/delete) works offline with zero setup regardless of any of the above — if reflection isn't working you can still just write.

## For developers

```
npm install                  # frontend deps
npm run dev                  # Vite dev server only (no Tauri window)
npm run tauri dev            # full app, hot-reloading
npm run build                # type-check + production frontend bundle
npm test                     # Vitest — frontend unit tests
cd src-tauri && cargo test   # Rust unit + integration tests
cd src-tauri && cargo check  # fast Rust type-check
cd src-tauri && cargo run --example retrieval_eval -- --embed-model nomic-embed-text
                             # retrieval eval against a live local Ollama
```

CI (`.github/workflows/ci.yml`) runs the frontend build + Vitest on
Ubuntu and `cargo test --all-targets` on Ubuntu and macOS for every push
and pull request.

Copy `.env.example` to `.env` if you want to override dev defaults
(Ollama host, log level). No secrets or API keys anywhere in this
project — see that file.

### Repository layout

```
src/                  React + TypeScript frontend (Vite)
  lib/ipc.ts           the only file that calls Tauri invoke() — typed command wrappers
  lib/schemas.ts        Zod validation mirrors of the Rust-side checks
  components/ui.tsx     Tailwind + Radix UI primitives (shadcn/ui-style, hand-owned not installed)
  screens/               one file per top-level screen
src-tauri/            Rust backend
  src/db/               SQLite connection pool, migrations, repo.rs (all SQL lives here)
  src/commands/          #[tauri::command] handlers — thin, call into db/repo, indexing, rag
  src/ollama/            loopback-only Ollama HTTP client
  src/indexing/          chunking + the background indexing worker
  src/rag/               retrieval (cosine similarity) + generation (structured output + validation)
  src/safety/            urgent-distress local pre-filter + bundled crisis response
  src/backup/            SQLite online-backup-API based backup/restore
  tests/                 Rust integration tests (temp SQLite DBs, no network)
  examples/retrieval_eval.rs   live-model retrieval eval harness
evals/fixtures/        fictional demo entries, labeled eval queries, safety test fixtures
evals/results/         eval reports written by the harness
tests/unit/            Vitest frontend tests
```

### A few design notes

- Every SQL statement lives in `src-tauri/src/db/repo.rs`. Commands, the
  indexing worker, and the RAG pipeline all call into it instead of
  writing ad-hoc queries, so there's one place that knows the schema.
- The frontend never calls `invoke()` directly outside `src/lib/ipc.ts`.
  New Tauri command → add its wrapper there too.
- `aggregate_version` and `embedding_space_version` are the two counters
  that make "a worry's outcome changed three weeks after it was indexed"
  work correctly — see `src-tauri/src/indexing/worker.rs` and
  `docs/spec.md`. `src-tauri/tests/retrieval_tests.rs` is basically the
  executable spec for this.
- The distress path (`src-tauri/src/safety/mod.rs`) is a keyword
  pre-filter, not a validated clinical triage system — I mean that
  literally, read that file's doc comment before touching it. It has two
  tiers: **imminent** skips the model entirely and returns a bundled
  crisis response; **elevated** still runs normal reflection, but adds a
  system instruction to check in on safety first and attaches a bundled
  (not model-written) support note, which survives even if the model call
  fails. Known false-positive/negative cases are in
  `evals/fixtures/safety_fixtures.json`.
- Reflect is multi-turn but still session-only: the frontend keeps the
  conversation in memory and sends recent turns with each message. Rust
  role-checks and trims them (last 8 turns, character-capped) before they
  reach the model, and the previous user turn is folded into the
  retrieval query so follow-ups like "what about the second one?" still
  find the right history.
- The webview can't choose file paths. Export, import, backup and restore
  open their native dialogs from Rust, and the capability file no longer
  grants the renderer dialog open/save permissions at all.

## Evaluating retrieval quality

`evals/fixtures/demo_entries.json` (20 fictional entries) and
`evals/fixtures/eval_queries.json` (8 labeled queries) drive an automated
retrieval eval:

```
ollama pull nomic-embed-text
cd src-tauri && cargo run --example retrieval_eval -- --embed-model nomic-embed-text
```

It seeds the fixtures into a throwaway vault, indexes them through the
app's real indexing code, and scores each query two ways: a naive top-k
baseline (raw cosine over everything, including the memory-disabled
entry) versus Anchor's retrieval as shipped (eligibility + version
filters, threshold, recency, linked-outcome expansion). It checks
recall, linked-outcome coverage, the no-match case, and that private
entries never leak, then writes a Markdown report to `evals/results/`.
The harness has been verified end to end against a stub server; I
haven't published a live-model run yet. Scoring generated responses
(not just retrieval) still needs a human read of transcripts; see
`evals/README.md`.

## Project status

Done: journal entries with search and tagging, full Worry Loop tracking
(worries, recorded outcomes, small steps), a local embedding/RAG pipeline
against Ollama with cited, multi-turn reflection and a natural-language
fallback when structured output isn't available, the two-tier distress
pre-filter, consent/memory controls, backup/export, vault erasure, an
automated retrieval eval harness, and CI. 34 passing Rust tests and 24
passing frontend tests (`cargo test` / `npm test`).

Not done yet: a signed/notarized macOS installer, database-level
encryption at rest, in-app model download, a proper usability study, and
a published live-model eval report (run the harness above to produce
one).

### Recent changes

- **Fixed:** deleting a worry outcome left its entry stuck as "pending"
  with no indexing job queued, so it silently dropped out of reflection.
  It's now re-queued like every other edit.
- **Fixed:** indexing retry backoff was recorded but never honored, so a
  failing job was retried every 3 seconds. The worker now waits out
  `next_attempt_at`, and a job in backoff no longer blocks others.
- **Fixed:** retrieval used the time a chunk was *indexed* for recency
  scoring and the date shown in the sources drawer, so rebuilding the
  index made every entry look new. It now uses when the entry was written.
- **Fixed:** two overlapping reflections could resume indexing while one
  was still running. Pausing is now reference-counted, separately from
  the manual pause in Settings.
- **New:** multi-turn Reflect, meaning follow-up questions keep their
  context.
- **New:** the "elevated" distress tier is now acted on, not just
  detected (see design notes above).
- **Hardened:** file dialogs moved into Rust, so the renderer never
  supplies a filesystem path.
- **New:** `retrieval_eval` harness, plus GitHub Actions CI on Ubuntu and
  macOS.

## License

Copyright © 2026 Arnav Thorat. All rights reserved. The source is public so
you can read it and evaluate the project, and you may build and run it for
personal, non-commercial use. Redistribution, modified or commercial use, or
presenting this work as your own is not permitted without written permission.
See [LICENSE](./LICENSE).
