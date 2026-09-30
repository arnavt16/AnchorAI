import { describe, expect, it, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { dataApi, reflectApi } from "@/lib/ipc";
import { toHistory } from "@/screens/Reflect";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("Reflect conversation history", () => {
  it("keeps real turns in order and drops error placeholders", () => {
    const history = toHistory([
      { role: "user", text: "I have two interviews this week" },
      { role: "assistant", text: "The local model runtime didn't respond.", failed: true },
      { role: "user", text: "still there?" },
      { role: "assistant", text: "Which one worries you more?" },
    ]);
    expect(history).toEqual([
      { role: "user", content: "I have two interviews this week" },
      { role: "user", content: "still there?" },
      { role: "assistant", content: "Which one worries you more?" },
    ]);
  });

  it("sends the history with the reflect command", async () => {
    mockInvoke.mockResolvedValueOnce({ sections: [] });
    const history = [{ role: "user" as const, content: "earlier" }];
    await reflectApi.reflect({ message: "what about the second one?", useMemory: true, history });
    expect(mockInvoke).toHaveBeenCalledWith("reflect", {
      input: { message: "what about the second one?", useMemory: true, history },
    });
  });
});

describe("file commands never send a path from the renderer", () => {
  it.each([
    ["exportJsonToFile", "export_vault_json_file"],
    ["pickImportFile", "pick_import_file"],
    ["exportMarkdown", "export_markdown"],
    ["backup", "backup_vault"],
    ["restore", "restore_vault"],
  ] as const)("%s → %s takes no arguments", async (fn, command) => {
    mockInvoke.mockResolvedValueOnce(null);
    await dataApi[fn]();
    expect(mockInvoke).toHaveBeenCalledWith(command, undefined);
  });
});
