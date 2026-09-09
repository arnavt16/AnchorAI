# Project status

Honest accounting of what's implemented, what's tested, what's verified,
and what's still deferred — per `docs/brief.md` deliverable #10 ("A final
summary distinguishing implemented features, automated tests, native
manual checks, signing status, and public-launch work"). This was written
by re-reading the actual code and test output, not from memory of intent.

## Build environment this was developed in

Everything in this repository was built and tested inside a Linux
(x86_64, Ubuntu 24.04) cloud sandbox with no macOS, no Xcode, no Ollama
runtime, and no ability to run a GUI interactively — only headless/offscreen
checks. The target platform (per your instruction) is macOS. Concretely,
that means:

- **Verified in this environment**: the Rust backend compiles cleanly on
  Linux (`cargo check`, `cargo build`, `cargo test` — see below), the
  frontend type-checks and builds (`tsc -b && vite build`), and the full
  Tauri binary was launched under Xvfb (a virtual, headless X server) for a
  smoke test — it opened a window, resolved the OS app-data directory,
  applied SQLite migrations, and created a working vault file with the
  correct schema, all without a crash. This is real evidence the
  application boots and the plumbing between Rust and the webview works —
  see "Smoke test details" below.
- **NOT verified in this environment, because it cannot be**: an actual
  macOS build, code signing/notarization, a live Ollama runtime (embedding
  generation, chat, indexing, reflection, the safety fixtures running
  against a real model), any interactive/visual QA of the UI, and
  installer packaging (`.dmg`).

You will need to run `npm run tauri build` on your own Mac (see README) to
get a real macOS bundle, and set up Ollama locally to exercise anything
AI-dependent. Nothing about the Linux-only verification here should be
read as "this works on macOS" — it's "this compiles and boots, on a
different OS, without a model runtime."

### Smoke test details

```
xvfb-run -a timeout 8 ./target/debug/anchor
```
Output showed the migration applying (`INFO anchor_lib::db: applied migration migration="0001_init"`)
and exited cleanly. Inspecting the resulting SQLite file afterward showed all
eight expected tables (`app_settings`, `journal_entries`, `worries`,
`worry_outcomes`, `small_steps`, `retrieval_chunks`, `indexing_jobs`, plus
the internal `_migrations` table). This confirms: app launch, window
creation, app-data-directory resolution, database creation, and migrations
— not reflection, indexing, or any Ollama-dependent path, none of which ran
because no Ollama was installed in this sandbox.

## Automated tests (all passing, actually run)

**Rust (`cd src-tauri && cargo test`): 20/20 passing**
- `tests/db_tests.rs` (7): entry persistence across simulated restart, empty-body rejection, cascade delete of worry/outcomes/steps, one-worry-per-entry enforcement, memory-disable immediately clearing chunks + cancelling jobs, outcome edits bumping the entry's aggregate version, and cross-entry worry/step mismatch rejection.
- `tests/chunker_tests.rs` (5): short text stays one chunk, empty text produces none, long text splits on paragraph boundaries within the word budget, an oversized single paragraph is hard-wrapped without dropping words, and a general no-content-loss check.
- `tests/retrieval_tests.rs` (5): memory-disabled entries never retrieved, different embedding-space chunks never match, different vault-generation chunks never match, cosine ranking actually ranks the more similar vector first, and a stale `aggregate_version` (edited-but-not-reindexed) chunk is excluded.
- `tests/safety_tests.rs` (3): all 10 labeled cases in `evals/fixtures/safety_fixtures.json` classify as expected (including a documented, accepted false positive — see that file's `s7` note and `SECURITY.md`), plus two standalone figurative-vs-literal checks.

**Frontend (`npm test`, Vitest): 17/17 passing**
- Zod schema validation (entry/worry/outcome/step forms).
- `src/lib/ipc.ts` error-mapping (Rust `SafeErrorCode` payloads become typed `AnchorApiError`) and command-name/argument correctness.
- Utility formatting functions.
- Component rendering/interaction for `Button`, `EmptyState`, `Badge`.

These are real unit/integration tests against real code paths (temp SQLite
files on disk for the Rust tests, real Zod parsing and real React rendering
via Testing Library for the frontend tests) — not placeholders.

**Not covered by automated tests**: anything requiring a live Ollama
runtime (actual embedding/chat calls, end-to-end indexing, end-to-end
reflection, the readiness-check flow), Tauri IPC permission enforcement
(would need a real packaged app + native test harness), and any visual/UI
behavior (focus states, keyboard nav, narrow-window layout) — these need
the manual checks described below, on your machine.

## Section-by-section status against `docs/brief.md`

| Area | Status |
| --- | --- |
| §1 Product concept | Implemented as specified |
| §3.A First visit / onboarding | Implemented (dialog on first launch, explains local-only storage, no signup) |
| §3.B Capture a thought | Implemented (title/body/mood/tags, memory toggle, unsaved-edit warning via `beforeunload`) |
| §3.C Create a worry | Implemented (one worry per entry, enforced in Rust) |
| §3.D Reflect | Implemented (three intentions, source drawer, session-only chat, explicit "Save my reflection") |
| §3.E Close the loop | Implemented (outcome form with result category + reflection, chronological, editable) |
| §3.F Small step | Implemented (action text + helped/did not help/unsure/not tried feedback) |
| §3.G Local model setup | Implemented: runtime detection, model selection from what's locally installed, synthetic readiness check (embedding dimension/finiteness + structured chat probe). **Not implemented**: in-app model *download* — the brief marks this optional for month one, and Setup's copy tells the user to `ollama pull` themselves. |
| §4 Screens | All six implemented (Home, Entry, Worry Loop, Reflect, Setup, Settings) |
| §5 Technical architecture | Tauri 2 + React 19 + TS + Vite + Tailwind + rusqlite (bundled SQLite) + reqwest-based Ollama client, as specified. Retrieval is exact cosine similarity, no vector extension. |
| §6 Storage/data model | All seven tables implemented with the exact fields, constraints, and foreign keys specified (see `src-tauri/migrations/0001_init.sql`) |
| §7 Durable indexing | Implemented: transactional canonical writes, background worker with bounded concurrency (one job at a time), resume-on-launch reclaiming of interrupted jobs, aggregate-version invalidation, stale-job discard, pause/resume controls, embedding-space migration with full re-index |
| §8 Retrieval/generation | Implemented: candidate retrieval → recency-weighted rerank → dedup → linked-outcome expansion → bounded context assembly → structured JSON generation → source-ID validation → one bounded repair attempt. If structured output still fails, Reflect asks the same model for a plain natural-language reply (no schema) instead of a canned apology — real conversation always wins over a dead end. `citationsVerified` on the response tells the UI whether a reply came from the cited/structured path or the natural-language fallback. The old apology-only fallback is now a true last resort, used only if even the plain-text call errors out. Post-generation recheck that cited sources are still current. |
| §9 Assistant behavior / urgent-distress | System prompt encodes the required behaviors (§9); urgent-distress routing implemented as a pre-filter with a bundled response — **requires human review before real use**, see SECURITY.md |
| §10 Desktop security | CSP set, capabilities scoped narrowly (no shell/arbitrary fs/generic HTTP), all SQL parameterized, no telemetry/logging of private text |
| §11 Backup/export/erasure | JSON export/import (with preview + transactional import), Markdown export, SQLite-online-API backup/restore with pre-restore safety snapshot, vault erasure. **Not yet run against a large/stress-sized vault.** |
| §13 Demo & evaluation | 20 fictional entries (brief asks for 20–30 — at the floor), 8 labeled eval queries, 10 labeled safety fixtures. The three-configuration retrieval comparison is designed and documented (`evals/README.md`) but **not run** — no Ollama in this build environment. |
| §14 Acceptance tests | Partially exercised by the automated test suite above; the full checklist (packaged-app install/relaunch, keyboard nav at narrow width, visual/UI checks) needs to be run manually on your Mac. |
| §16 Packaging | **Not done.** No signed/notarized artifact exists. See README for how to build one yourself; expect a Gatekeeper warning on an ad-hoc/unsigned build. |
| §18 Month-one milestone | The core list is implemented; deferred items (full usability study with volunteers, in-app model download, additional model choices, multi-platform installers) are genuinely deferred, not stubbed-and-hidden. |

## Known gaps / what to do next

1. **Run it on an actual Mac.** This is the single highest-value next step — everything above the "Linux smoke test" line is unverified on the target platform.
2. **Install Ollama and run the eval queries** in `evals/` by hand, and read the transcripts — especially the prompt-injection (`q6`) and unfavorable-outcome-honesty (`q4`) cases.
3. **Human-review the safety subsystem** (`src-tauri/src/safety/mod.rs` + `evals/fixtures/safety_fixtures.json`) against real model behavior before showing this to anyone in a context where it might matter. The crisis-resource list also needs real verification — it's currently marked unverified in the code itself.
4. **Stress-test indexing/retrieval at scale** (the brief suggests ~10,000 chunks) — only exercised here at fixture scale (20 entries).
5. Packaging, signing, and the volunteer usability walkthrough are explicitly deferred per the brief's own "features that may follow the month-one milestone" list.

Nothing above is a placeholder pretending to be finished. Where something
isn't done, it's listed as not done.
