# Anchor

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
embedding/RAG pipeline against Ollama, structured reflection with source
citations, and the safety/privacy plumbing (consent, memory exclusion,
urgent-distress routing, session-only chat). Packaging a signed
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

I developed and smoke-tested this on Linux — it hasn't been packaged or
run on macOS yet (see [Project status](#project-status)). To build it
yourself on a Mac:

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
```

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
evals/fixtures/        fictional demo entries, labeled eval queries, safety test fixtures
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
- The urgent-distress path (`src-tauri/src/safety/mod.rs`) is a keyword
  pre-filter, not a validated clinical triage system — I mean that
  literally, read that file's doc comment before touching it. Known
  false-positive/negative cases are in `evals/fixtures/safety_fixtures.json`.

## Evaluating retrieval quality

`evals/fixtures/demo_entries.json` and `evals/fixtures/eval_queries.json`
are a small labeled fixture set for checking retrieval quality by hand
(see `evals/README.md`). The full three-way comparison in `docs/spec.md`
(no-retrieval vs. plain top-k vs. Anchor's version-aware retrieval) needs
a running local Ollama with models pulled — the fixtures and harness are
ready, I just haven't run the live-model pass myself yet.

## Project status

Done: journal entries with search and tagging, full Worry Loop tracking
(worries, recorded outcomes, small steps), a local embedding/RAG pipeline
against Ollama with cited reflection responses and a natural-language
fallback when structured output isn't available, the urgent-distress
safety pre-filter, consent/memory controls, backup/export, and vault
erasure. 20 passing Rust integration tests and 17 passing frontend unit
tests (`cargo test` / `npm test`).

Not done yet: a signed/notarized macOS installer, database-level
encryption at rest, in-app model download, a proper usability study, and
running the retrieval eval fixtures against a live model (the harness is
ready — see `evals/README.md`). Developed and smoke-tested on Linux only
so far.
