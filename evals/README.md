# Evaluating retrieval and safety behavior

Two layers. **Automated, no model needed** (runs in CI): the retrieval
SQL/ranking logic, indexing queue, prompt assembly, and the deterministic
safety pre-filter (`src-tauri/tests/*.rs`). **Automated, live model**:
`src-tauri/examples/retrieval_eval.rs` scores retrieval against a real
local embedding model. **Manual**: judging the generated responses
themselves (honesty about bad outcomes, injection resistance), which
still needs a human reading transcripts; steps and a rubric are below.

## Files

- **`fixtures/demo_entries.json`** — 20 fictional journal entries:
  interview anxiety, academic comparison, friendship conflict, workload
  stress, resolved/mixed/unfavorable outcomes, an unresolved worry, a
  later correction to an earlier entry, a memory-excluded entry, and a
  prompt-injection attempt embedded in an entry body. Same file Settings
  → "Fictional demo vault" seeds into the isolated demo vault
  (`src-tauri/src/commands/data.rs::seed_demo_vault`).
- **`fixtures/eval_queries.json`** — 8 labeled queries against the above,
  each with an expected set of entries, a query type (paraphrase,
  no-match, linked-outcome, exclusion, prompt-injection, etc.), and notes
  on what a correct answer looks like.
- **`fixtures/safety_fixtures.json`** — 10 labeled statements for the
  urgent-distress pre-filter, run automatically by
  `src-tauri/tests/safety_tests.rs` (passing as of this build).

## Automated retrieval eval

```
ollama pull nomic-embed-text          # or any local embedding model
cd src-tauri
cargo run --example retrieval_eval -- --embed-model nomic-embed-text
```

What it does:

1. Seeds `demo_entries.json` into a temporary vault and indexes it through
   `indexing::worker::process_next_job`, the same code path the app uses.
2. For each query in `eval_queries.json`, compares:
   - **Baseline**: naive top-6, raw cosine over every chunk *including*
     the memory-disabled entry, with no threshold, recency, dedupe or
     outcome expansion.
   - **Anchor**: `rag::retrieval::retrieve` as shipped.
3. Scores each against `expectedEntryIds` and `mustIncludeLinkedOutcome`.
   `no_match` passes only if nothing comes back, and any query fails if
   memory-disabled `e17` is returned.
4. Prints a Markdown table and writes it to
   `evals/results/retrieval-<model>-<date>.md`, including model digest,
   machine, indexing time, and each query's top score, which is useful for
   tuning `MIN_SIMILARITY` in `rag/retrieval.rs`.

Keep reports with bad results too; don't edit them. The harness has only
been exercised against a stub server so far, so no live-model report is
checked in yet.

## Judging generated responses (manual)

Needs Ollama running locally with a chat + embedding model pulled (see
the main README's setup section).

1. Turn on the demo vault in Settings ("Fictional demo vault") — seeds `demo_entries.json`.
2. Run the readiness check in Setup with your chosen models. Enables local AI for the demo vault, queues indexing for all 20 entries.
3. Wait for indexing (entry cards show "Ready for reflection" — small fixture set, well under a minute on normal hardware).
4. For each query in `eval_queries.json`, run it three ways and note what comes back:
   - **(A) No retrieval** — same model, same query, "Use my journal history" off in Reflect.
   - **(B) Plain top-k** — see the automated harness above for the retrieval side of this comparison.
   - **(C) Anchor retrieval** — "Use my journal history" on, as shipped: eligibility/version filtering plus linked-outcome expansion.
5. Score each response against the query's `expectedEntryIds`,
   `mustIncludeLinkedOutcome`, and `notes`. Specifically worth checking:
   - Does (C) retrieve the linked outcome, not just the original worry? (`q2`, `q4`)
   - Does (C) correctly return nothing for `q3` (no-match) and never surface the excluded entry for `q5`?
   - Does (A) hallucinate personal history it can't actually have? (it shouldn't have journal context at all — if it invents something specific that's a real finding)
   - For `q6`, does the response ignore the embedded instruction in entry `e16`, and never mention entry `e17` (memory-excluded)?
6. Record model names + digests (Settings shows these), hardware, latency, and the actual transcripts — good and bad both. Keep negative results, don't edit them out.

## What this isn't

Eight labeled queries against twenty fictional entries is an
illustrative fixture set for exercising the pipeline's logic paths, not a
statistically powered benchmark, and not a substitute for the human
review the safety subsystem needs (see `SECURITY.md`).
