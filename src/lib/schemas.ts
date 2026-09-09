import { z } from "zod";

// Client-side validation mirrors (not replaces) the Rust-side checks in
// src-tauri/src/db/repo.rs. The renderer never gets to skip validation just
// because these pass — every IPC command re-validates in Rust.

export const entryFormSchema = z.object({
  title: z.string().max(200).optional(),
  body: z.string().min(1, "Write something first.").max(200_000, "That entry is too long."),
  mood: z.string().max(40).optional(),
  tags: z.array(z.string().max(40)).max(20),
  memoryEnabled: z.boolean(),
});
export type EntryFormValues = z.infer<typeof entryFormSchema>;

export const worryFormSchema = z.object({
  worryText: z.string().min(1, "Say what the worry is, in your own words.").max(4000),
  expectedOutcome: z.string().max(4000).optional(),
});
export type WorryFormValues = z.infer<typeof worryFormSchema>;

export const outcomeFormSchema = z.object({
  outcomeText: z.string().min(1, "Describe what actually happened.").max(4000),
  resultCategory: z
    .enum(["better_than_expected", "about_as_expected", "harder_than_expected", "mixed", "still_unsure"])
    .optional(),
  reflection: z.string().max(4000).optional(),
});
export type OutcomeFormValues = z.infer<typeof outcomeFormSchema>;

export const stepFormSchema = z.object({
  actionText: z.string().min(1, "Describe the small step.").max(1000),
});
export type StepFormValues = z.infer<typeof stepFormSchema>;

export const reflectFormSchema = z.object({
  message: z.string().min(1).max(8000),
  intention: z.enum(["unpack", "related_experience", "small_step"]).optional(),
  useMemory: z.boolean(),
});
export type ReflectFormValues = z.infer<typeof reflectFormSchema>;
