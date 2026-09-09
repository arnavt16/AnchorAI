# Anchor

Anchor is a local-first desktop journal. Its distinguishing feature, **Worry
Loop**, connects a worry you're having right now to relevant past worries,
the outcomes you recorded for them, and the things you said helped —
retrieved and generated entirely on your own computer.

Anchor is an applied LLM/RAG engineering portfolio project, not a clinical
product. It is not clinically validated, does not diagnose, and is not a
substitute for professional care. See [SECURITY.md](./SECURITY.md) for the
full privacy and threat-model writeup, and [PROJECT_STATUS.md](./PROJECT_STATUS.md)
for exactly what's implemented, tested, and still deferred.

## What's here right now

This repository is the Phase 1–4 core described in `docs/brief.md`'s
implementation plan (see section 18, "month-one milestone"): a working
desktop shell, full journal + Worry Loop CRUD, a real local embedding/RAG
pipeline against Ollama, structured reflection with source citations, and
the safety/privacy plumbing (consent, memory exclusion, urgent-distress
routing, session-only chat). Packaging a signed installer, the full
usability study, and a few convenience features are **not** done yet — see
`PROJECT_STATUS.md`.

## For end users: installing and running Anchor

Anchor needs two separate pieces of software: **Anchor itself**, and
**Ollama**, which runs the local AI models Anchor's reflection feature uses.
You can use Anchor as a plain journal without Ollama at all.

### 1. Install Ollama (optional, only needed for reflection)

1. Download Ollama from [ollama.com](https://ollama.com) for macOS and install it normally.
2. Open a terminal and pull one chat model and one embedding model, for example:
   ```
   ollama pull llama3.1:8b
   ollama pull nomic-embed-text
   ```
   These are suggestions, not requirements — any local chat + embedding
   model pair Ollama can run will work; smaller or larger models trade off
   speed against reflection quality on your hardware.
3. Make sure Ollama is running (it usually starts automatically after install; `ollama list` in a terminal should show your pulled models).
4. For the strongest local-only guarantee, also enable Ollama's documented cloud-disabled setting — see [Ollama's FAQ](https://docs.ollama.com/faq). Anchor's own network client refuses any non-loopback host regardless, but Ollama itself can proxy to cloud-hosted models under some configurations, and that's a setting on Ollama's side, not Anchor's.

### 2. Get Anchor running

This build was developed and smoke-tested on Linux (the environment used to build it) and has **not yet been packaged or run on macOS** — see `PROJECT_STATUS.md` for exactly what "smoke-tested" means here. To build and run it on your Mac:

1. Install [Node.js](https://nodejs.org) (v20+) and [Rust](https://www.rust-lang.org/tools/install) (`curl https://sh.rustup.rs -sSf | sh`).
2. Install Xcode Command Line Tools: `xcode-select --install`.
3. From this project's folder:
   ```
   npm install
   npm run tauri build
   ```
   The first build compiles Rust dependencies and can take several minutes. The finished app bundle will be under `src-tauri/target/release/bundle/macos/` (and a `.dmg` under `bundle/dmg/` if that target succeeds on your machine).
4. Open the built `Anchor.app`. On first launch, macOS Gatekeeper will likely warn that the app is from an unidentified developer (it isn't signed or notarized — see `PROJECT_STATUS.md`). Right-click the app and choose "Open" to bypass this once, or allow it in System Settings → Privacy & Security.
5. To iterate on the app without a full release build, use `npm run tauri dev` instead of step 3.

Your journal lives in this Mac's per-user Application Support directory
(shown in Settings inside the app once it's running), not inside the
installed app bundle — deleting or reinstalling the app does not touch it.

### If something's not working

- **"Local AI unavailable" in Setup**: confirm `ollama list` shows your models in a terminal, and that nothing else is using port 11434.
- **App won't open at all on macOS**: this usually means Gatekeeper — see step 4 above.
- **Reflection is slow or times out**: try a smaller chat model; performance depends heavily on your Mac's RAM and whether you're on Apple Silicon.
- Anchor's journal (create/edit/search/delete) works with zero setup and no internet connection, independent of any of the above — if reflection isn't working, you can still just write.

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

Environment: copy `.env.example` to `.env` if you want to override dev
defaults (Ollama host, log level). See that file — there are no secrets or
API keys involved anywhere in this project.

### Repository layout

```
src/                  React + TypeScript frontend (Vite)
  lib/ipc.ts           the ONLY file that calls Tauri invoke() — typed command wrappers
  lib/schemas.ts        Zod validation mirrors of the Rust-side checks
  components/ui.tsx     Tailwind + Radix UI primitives (shadcn/ui-style, hand-owned not installed)
  screens/               one file per top-level screen
src-tauri/            Rust backend
  src/db/               SQLite connection pool, migrations, repo.rs (all SQL lives here)
  src/commands/          #[tauri::command] handlers — thin, call into db/repo, indexing, rag
  src/ollama/            loopback-only Ollama HTTP client
  src/indexing/          chunking + the durable background indexing worker
  src/rag/               retrieval (cosine similarity) + generation (structured output + validation)
  src/safety/            urgent-distress local pre-filter + bundled crisis response
  src/backup/            SQLite online-backup-API based backup/restore
  tests/                 Rust integration tests (temp SQLite DBs, no network)
evals/fixtures/        fictional demo entries, labeled eval queries, safety test fixtures
tests/unit/            Vitest frontend tests
```

### Design notes worth knowing before you read the code

- **Every SQL statement lives in `src-tauri/src/db/repo.rs`.** Commands, the indexing worker, and the RAG pipeline all call into it rather than writing ad-hoc queries, so there's exactly one place that knows the schema shape.
- **The frontend never calls `invoke()` directly** outside `src/lib/ipc.ts`. If you're adding a new Tauri command, add its wrapper there too.
- **`aggregate_version` and `embedding_space_version`** are the two version counters that make "a worry's outcome changed three weeks after the worry was indexed" work correctly — see `src-tauri/src/indexing/worker.rs`'s module doc and `docs/brief.md` section 7 for the full reasoning. The retrieval tests in `src-tauri/tests/retrieval_tests.rs` are the executable spec for this.
- **The urgent-distress path (`src-tauri/src/safety/mod.rs`) is a keyword pre-filter, explicitly not a validated clinical triage system.** Read that file's module doc before changing it, and see `evals/fixtures/safety_fixtures.json` for its known false-positive/false-negative tradeoffs.

## Evaluating retrieval quality

`evals/fixtures/demo_entries.json` and `evals/fixtures/eval_queries.json` are
a small labeled fixture set for manually or programmatically evaluating
retrieval quality (see `evals/README.md`). Running the actual three-way
comparison described in the brief (no-retrieval vs. plain top-k vs.
Anchor's version-aware retrieval) requires a running local Ollama with
models pulled — that hasn't been run as part of building this repository,
since this build environment has no Ollama runtime installed. The fixtures
and harness design are ready for you to run this yourself; see
`evals/README.md` for exact steps and `PROJECT_STATUS.md` for what's been
verified here versus what's left for you to check on your own machine.
