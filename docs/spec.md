# Design notes

Condensed version of the design notes I worked from while building
Anchor. Not a full spec document, just the decisions worth writing down
so I (or anyone reading the code) can tell why things are shaped the way
they are. The section references in code comments and other docs point
back to the topics below.

## Overview

Anchor is a desktop app for private journaling and memory-grounded
reflection. The feature I actually cared about proving out — Worry
Loop — connects a current worry to relevant past worries, the outcomes
I recorded for them, and whatever I said helped at the time.

Stack: Tauri (Rust backend, React/TypeScript frontend), SQLite, local
Ollama models. No cloud database, no backend service, no hosted auth, no
API keys, no cloud AI fallback. Storage, embeddings, retrieval, and
inference all happen on-device.

## Constraints I set for myself

Build it without publishing anywhere, buying certificates, or standing up
external accounts. Ollama and any models are a separate install the user
does themselves — nothing downloads multi-gigabyte models or changes a
shared config silently. Where I couldn't verify something in my own dev
environment (no signed build, no macOS test machine at the time), I
wanted that documented honestly instead of claimed.

I only claim platforms I've actually tested. Linux is what I built and
smoke-tested on; the source is portable to macOS/Windows but I'm not
calling those "supported" until I've verified them. End users shouldn't
need Node, Rust, or a terminal to run the packaged app — that's a
separate concern from what a dev checkout needs.

## Framing

This is a portfolio prototype for reflection and support, not a clinical
product. It doesn't claim diagnosis, therapy efficacy, HIPAA compliance,
guaranteed security, or comprehensive crisis detection. If this were ever
headed toward a real release, the model behavior and desktop security
would both need a proper human review pass first — see `SECURITY.md`.

## Where the rest of this lives

I didn't keep a monolithic spec document — the architecture reasoning
lives close to the code it explains: screen behavior in the screen
components themselves, the storage schema in
`src-tauri/src/db/repo.rs` and the migration files, the indexing
lifecycle in `src-tauri/src/indexing/worker.rs`'s module doc, the
retrieval/generation pipeline in `src-tauri/src/rag/`, security
requirements in `SECURITY.md`, and the eval fixtures/rubric in
`evals/README.md`. If you're trying to understand a specific piece,
that's usually a better read than a top-down spec would be.
