import { type ClassValue, clsx } from "clsx";

export function cn(...inputs: ClassValue[]) {
  return clsx(inputs);
}

export function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  } catch {
    return iso;
  }
}

export function formatDateTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
  } catch {
    return iso;
  }
}

export function truncate(text: string, max = 140): string {
  const t = text.trim();
  return t.length > max ? `${t.slice(0, max).trimEnd()}…` : t;
}

const RESULT_LABELS: Record<string, string> = {
  better_than_expected: "Better than expected",
  about_as_expected: "About as expected",
  harder_than_expected: "Harder than expected",
  mixed: "Mixed",
  still_unsure: "Still unsure",
};
export function resultCategoryLabel(v: string | null | undefined): string | null {
  if (!v) return null;
  return RESULT_LABELS[v] ?? v;
}

const INDEXING_LABELS: Record<string, string> = {
  excluded: "Not used for reflection",
  pending: "Preparing memories…",
  processing: "Preparing memories…",
  ready: "Ready for reflection",
  failed: "Memory processing failed",
};
export function indexingStatusLabel(v: string): string {
  return INDEXING_LABELS[v] ?? v;
}
