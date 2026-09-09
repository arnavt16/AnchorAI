import { describe, expect, it } from "vitest";
import { truncate, resultCategoryLabel, indexingStatusLabel } from "@/lib/utils";

describe("truncate", () => {
  it("leaves short text unchanged", () => {
    expect(truncate("short text", 140)).toBe("short text");
  });

  it("truncates long text with an ellipsis", () => {
    const long = "word ".repeat(100);
    const result = truncate(long, 20);
    expect(result.length).toBeLessThanOrEqual(21);
    expect(result.endsWith("…")).toBe(true);
  });
});

describe("resultCategoryLabel", () => {
  it("maps known categories to readable labels", () => {
    expect(resultCategoryLabel("harder_than_expected")).toBe("Harder than expected");
    expect(resultCategoryLabel(null)).toBeNull();
  });
});

describe("indexingStatusLabel", () => {
  it("never claims 'ready' for a pending entry", () => {
    expect(indexingStatusLabel("pending")).not.toMatch(/ready/i);
    expect(indexingStatusLabel("ready")).toMatch(/ready/i);
  });
});
