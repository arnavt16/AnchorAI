import "@testing-library/jest-dom/vitest";

// Anchor's renderer talks to Tauri only through src/lib/ipc.ts, which wraps
// @tauri-apps/api/core's `invoke`. That module doesn't exist outside a real
// Tauri webview, so unit tests that import ipc.ts (even transitively)
// need a stub. Individual test files override this with vi.mock when they
// need specific return values.
import { vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.reject(new Error("invoke() called without a test-specific mock"))),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
  open: vi.fn(),
  confirm: vi.fn(),
  message: vi.fn(),
}));
