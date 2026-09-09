/**
 * Typed wrappers around Anchor's narrow Tauri commands (see
 * src-tauri/src/commands/*.rs). This is the ONLY place the frontend talks
 * to `invoke`. Screens and components import from here, never call
 * `@tauri-apps/api/core` directly, so the IPC surface stays auditable in
 * one file.
 */
import { invoke } from "@tauri-apps/api/core";

export type SafeErrorCode =
  | "not_found"
  | "invalid_input"
  | "database_locked"
  | "database_error"
  | "runtime_stopped"
  | "model_missing"
  | "model_incompatible"
  | "local_only_unverified"
  | "cancelled"
  | "insufficient_disk"
  | "malformed_import"
  | "consent_required"
  | "vault_generation_stale"
  | "embedding_space_mismatch"
  | "unexpected";

export class AnchorApiError extends Error {
  code: SafeErrorCode;
  constructor(code: SafeErrorCode, message: string) {
    super(message);
    this.code = code;
    this.name = "AnchorApiError";
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (e && typeof e === "object" && "code" in (e as any) && "message" in (e as any)) {
      const err = e as { code: SafeErrorCode; message: string };
      throw new AnchorApiError(err.code, err.message);
    }
    throw e;
  }
}

// ---------- Domain types (mirror src-tauri/src/models.rs) ----------

export interface JournalEntry {
  id: string;
  title: string | null;
  body: string;
  mood: string | null;
  tags: string[];
  origin: "user" | "imported" | "saved_reflection";
  memoryEnabled: boolean;
  contentVersion: number;
  aggregateVersion: number;
  indexingStatus: "excluded" | "pending" | "processing" | "ready" | "failed";
  createdAt: string;
  updatedAt: string;
}

export interface Worry {
  id: string;
  entryId: string;
  worryText: string;
  expectedOutcome: string | null;
  status: "open" | "resolved" | "archived";
  createdAt: string;
  updatedAt: string;
}

export interface WorryOutcome {
  id: string;
  worryId: string;
  recordedAt: string;
  outcomeText: string;
  resultCategory: string | null;
  reflection: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SmallStep {
  id: string;
  entryId: string;
  worryId: string | null;
  actionText: string;
  feedback: "helped" | "did_not_help" | "unsure" | "not_tried" | null;
  feedbackNote: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface WorryWithHistory extends Worry {
  outcomes: WorryOutcome[];
  steps: SmallStep[];
}

export interface AppSettings {
  localAiEnabled: boolean;
  consentVersion: string | null;
  consentAt: string | null;
  consentWithdrawnAt: string | null;
  chatModel: string | null;
  chatModelDigest: string | null;
  embeddingModel: string | null;
  embeddingModelDigest: string | null;
  embeddingDimension: number | null;
  embeddingSpaceVersion: number;
  supportCountry: string | null;
  vaultGeneration: number;
}

export interface RetrievedSource {
  id: string;
  entryId: string;
  sourceKind: string;
  content: string;
  date: string;
  similarity: number;
  linkedOutcome: WorryOutcome | null;
}

export interface ReflectionSection {
  text: string;
  sourceIds: string[];
}

export interface ValidatedReflection {
  sections: ReflectionSection[];
  followUpQuestion: string | null;
  suggestedStep: string | null;
  sources: RetrievedSource[];
  usedMemory: boolean;
  urgentPathTriggered: boolean;
  /** True only for the structured, cited path. False for natural-language
   * fallback replies and the urgent-distress path (still a real answer,
   * just not schema-verified). */
  citationsVerified: boolean;
}

export type VaultMode = "personal" | "demo";

// ---------- Entries ----------

export const entriesApi = {
  save: (input: { title?: string; body: string; mood?: string; tags: string[]; memoryEnabled: boolean }) =>
    call<JournalEntry>("save_entry", { input }),
  get: (id: string) => call<JournalEntry | null>("get_entry", { id }),
  list: (search?: string) => call<JournalEntry[]>("list_entries", { search }),
  update: (id: string, input: { title?: string; body: string; mood?: string; tags: string[] }) =>
    call<JournalEntry>("update_entry", { id, input }),
  delete: (id: string) => call<void>("delete_entry", { id }),
  setMemoryEligibility: (id: string, enabled: boolean) => call<JournalEntry>("set_memory_eligibility", { id, enabled }),
};

// ---------- Worries / outcomes / steps ----------

export const worriesApi = {
  create: (entryId: string, worryText: string, expectedOutcome?: string) =>
    call<Worry>("create_worry", { entryId, worryText, expectedOutcome }),
  update: (id: string, worryText: string, expectedOutcome?: string) =>
    call<Worry>("update_worry", { id, worryText, expectedOutcome }),
  list: (status?: "open" | "resolved" | "archived") => call<WorryWithHistory[]>("list_worries", { status }),
  setStatus: (id: string, status: "open" | "resolved" | "archived") => call<Worry>("set_worry_status", { id, status }),
  createOutcome: (worryId: string, outcomeText: string, resultCategory?: string, reflection?: string) =>
    call<WorryOutcome>("create_outcome", { worryId, outcomeText, resultCategory, reflection }),
  updateOutcome: (id: string, outcomeText: string, resultCategory?: string, reflection?: string) =>
    call<WorryOutcome>("update_outcome", { id, outcomeText, resultCategory, reflection }),
  deleteOutcome: (id: string) => call<void>("delete_outcome", { id }),
  createStep: (entryId: string, actionText: string, worryId?: string) =>
    call<SmallStep>("create_step", { entryId, worryId, actionText }),
  setStepFeedback: (id: string, feedback?: string, note?: string) =>
    call<SmallStep>("set_step_feedback", { id, feedback, note }),
};

// ---------- System / settings / model runtime ----------

export interface RuntimeStatus {
  reachable: boolean;
  localModels: string[];
}

export interface ReadinessResult {
  embeddingOk: boolean;
  embeddingDimension: number | null;
  embeddingError: string | null;
  chatOk: boolean;
  chatError: string | null;
}

export const systemApi = {
  getSettings: () => call<AppSettings>("get_settings"),
  grantConsent: () => call<AppSettings>("grant_local_ai_consent"),
  withdrawConsent: () => call<AppSettings>("withdraw_local_ai_consent"),
  setSupportCountry: (country?: string) => call<AppSettings>("set_support_country", { country }),
  checkRuntime: () => call<RuntimeStatus>("check_ollama_runtime"),
  runReadinessCheck: (chatModel: string, embeddingModel: string) =>
    call<ReadinessResult>("run_readiness_check_and_select", { chatModel, embeddingModel }),
  pauseIndexing: () => call<void>("pause_indexing"),
  resumeIndexing: () => call<void>("resume_indexing"),
  rebuildIndex: () => call<number>("rebuild_index"),
};

// ---------- Reflection ----------

export const reflectApi = {
  reflect: (input: { message: string; intention?: string; useMemory: boolean; excludeEntryId?: string }) =>
    call<ValidatedReflection>("reflect", { input }),
  saveAsEntry: (text: string, tags: string[]) => call<JournalEntry>("save_reflection_as_entry", { input: { text, tags } }),
};

// ---------- Export / import / backup / demo ----------

export interface ImportPreview {
  entryCount: number;
  worryCount: number;
  outcomeCount: number;
  stepCount: number;
}

export const dataApi = {
  exportJson: () => call<string>("export_vault_json"),
  writeTextExport: (path: string, content: string) => call<void>("write_text_export", { path, content }),
  previewImport: (json: string) => call<ImportPreview>("preview_vault_import", { json }),
  importJson: (json: string) => call<ImportPreview>("import_vault_json", { json }),
  readTextImport: (path: string) => call<string>("read_text_import", { path }),
  exportMarkdown: (targetDir: string) => call<number>("export_markdown", { targetDir }),
  backup: (targetPath: string) => call<void>("backup_vault", { targetPath }),
  restore: (sourcePath: string) => call<void>("restore_vault", { sourcePath }),
  erase: () => call<void>("erase_vault"),
  getVaultMode: () => call<VaultMode>("get_vault_mode"),
  switchVaultMode: (mode: VaultMode) => call<VaultMode>("switch_vault_mode", { mode }),
  seedDemoVault: () => call<number>("seed_demo_vault"),
};
