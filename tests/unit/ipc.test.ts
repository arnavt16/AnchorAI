import { describe, expect, it, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { entriesApi, AnchorApiError } from "@/lib/ipc";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("ipc error mapping", () => {
  it("wraps a Rust safe-error payload into an AnchorApiError", async () => {
    mockInvoke.mockRejectedValueOnce({ code: "invalid_input", message: "Entry body is required." });
    let caught: unknown;
    try {
      await entriesApi.save({ body: "", tags: [], memoryEnabled: false });
    } catch (e) {
      caught = e;
    }
    expect(caught).toBeInstanceOf(AnchorApiError);
    expect((caught as AnchorApiError).code).toBe("invalid_input");
  });

  it("passes through an unexpected non-safe-error rejection", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("boom"));
    await expect(entriesApi.list()).rejects.toThrow("boom");
  });

  it("calls the expected command name and args", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await entriesApi.list("worry");
    expect(mockInvoke).toHaveBeenCalledWith("list_entries", { search: "worry" });
  });
});
