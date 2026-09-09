# Anchor: design specification

This is the working specification Anchor was built against — reproduced
here for reference. Sections 1–18 (product concept, scope, user journeys,
screens, technical architecture, storage/data model, indexing lifecycle,
retrieval/generation pipeline, assistant behavior, desktop security,
backup/export/erasure, repository structure, demo/evaluation, acceptance
tests, implementation phases, packaging, final deliverables, and the
one-month portfolio milestone) are summarized rather than reproduced in
full here to keep this file a reasonable size — the section numbers
referenced throughout this repository's code comments and other docs
match this numbering.

## Overview

Anchor is a downloadable desktop application for private journaling and
memory-grounded mental wellness reflection. Its defining feature, Worry
Loop, connects a current worry to relevant past worries, user-recorded
outcomes, and things the user said helped.

The architecture is a standalone Tauri application: React, TypeScript,
SQLite, and local Ollama models. There is no cloud database, website
backend, hosted authentication, API key requirement, or automatic cloud
AI fallback. Journal storage, embeddings, retrieval, and AI inference all
happen on the user's device.

## Build constraints

Build locally without publishing, buying certificates, or creating
external accounts. Users install Anchor and Ollama separately and
download supported models through guided setup. Nothing installs
software silently, changes a shared Ollama configuration, or downloads
multi-gigabyte models without an explicit action. Where models or build
tools aren't available in a given environment, the real integration
still ships alongside a clearly labeled fictional demo mode, and any
unverified checks are reported honestly rather than assumed.

The host operating system is the initial verified build target. The
source stays portable across macOS, Windows, and Linux, but only
platforms actually tested are claimed as supported. End users don't need
Node.js, Rust, a terminal, or a development server to launch the packaged
app — developer build prerequisites are a separate concern from the
packaged release.

## Product framing

This is an adult-oriented portfolio prototype for reflection and support,
not clinically validated treatment. It does not claim diagnosis, therapy
efficacy, HIPAA compliance, guaranteed security, or comprehensive crisis
detection. Human review of model behavior and desktop security is
required before any public mental-health-facing release.

*(The full section-by-section specification — screens, storage schema,
indexing lifecycle, the retrieval/generation pipeline, assistant
behavior, security requirements, backup/export/erasure, evaluation
fixtures, acceptance tests, phases, and packaging — is summarized
throughout this codebase's own comments and the other docs in this
`docs/` directory, particularly `SECURITY.md` and `evals/README.md`.)*
