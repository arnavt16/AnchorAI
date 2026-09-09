import { describe, expect, it } from "vitest";
import { entryFormSchema, outcomeFormSchema, stepFormSchema, worryFormSchema } from "@/lib/schemas";

describe("entryFormSchema", () => {
  it("rejects an empty body", () => {
    const result = entryFormSchema.safeParse({ body: "", tags: [], memoryEnabled: false });
    expect(result.success).toBe(false);
  });

  it("accepts a minimal valid entry", () => {
    const result = entryFormSchema.safeParse({ body: "Something on my mind.", tags: [], memoryEnabled: false });
    expect(result.success).toBe(true);
  });

  it("rejects more than 20 tags", () => {
    const tags = Array.from({ length: 21 }, (_, i) => `tag${i}`);
    const result = entryFormSchema.safeParse({ body: "text", tags, memoryEnabled: false });
    expect(result.success).toBe(false);
  });
});

describe("worryFormSchema", () => {
  it("requires worry text", () => {
    expect(worryFormSchema.safeParse({ worryText: "" }).success).toBe(false);
    expect(worryFormSchema.safeParse({ worryText: "I might fail" }).success).toBe(true);
  });
});

describe("outcomeFormSchema", () => {
  it("only accepts known result categories", () => {
    expect(outcomeFormSchema.safeParse({ outcomeText: "it happened", resultCategory: "mixed" }).success).toBe(true);
    expect(outcomeFormSchema.safeParse({ outcomeText: "it happened", resultCategory: "great!" as any }).success).toBe(false);
  });
});

describe("stepFormSchema", () => {
  it("requires non-empty action text", () => {
    expect(stepFormSchema.safeParse({ actionText: "" }).success).toBe(false);
    expect(stepFormSchema.safeParse({ actionText: "Take a walk" }).success).toBe(true);
  });
});
