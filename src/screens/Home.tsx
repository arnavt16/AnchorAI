import { useEffect, useState } from "react";
import { Plus } from "lucide-react";
import { useAppState } from "@/lib/state";
import { entriesApi, worriesApi, type JournalEntry, type WorryWithHistory } from "@/lib/ipc";
import { Button, Card, CardContent, EmptyState, Badge, Spinner } from "@/components/ui";
import { formatDate, truncate, indexingStatusLabel } from "@/lib/utils";

export function HomeScreen() {
  const { navigate, pushToast } = useAppState();
  const [entries, setEntries] = useState<JournalEntry[] | null>(null);
  const [openWorries, setOpenWorries] = useState<WorryWithHistory[] | null>(null);

  useEffect(() => {
    load();
  }, []);

  async function load() {
    try {
      const [e, w] = await Promise.all([entriesApi.list(), worriesApi.list("open")]);
      setEntries(e);
      setOpenWorries(w);
    } catch (err) {
      pushToast("Couldn't load your journal right now.", "error");
    }
  }

  return (
    <div className="mx-auto max-w-2xl px-8 py-10">
      <header className="mb-8">
        <h1 className="text-xl font-semibold tracking-tight">A place to write freely</h1>
        <p className="mt-1 text-sm text-muted dark:text-muted-dark">
          Reflect with context from your own journal — stored on this computer only.
        </p>
      </header>

      <button
        onClick={() => navigate({ name: "newEntry" })}
        className="mb-8 flex w-full items-center gap-3 rounded-2xl border border-line dark:border-line-dark bg-surface dark:bg-surface-dark px-5 py-4 text-left shadow-soft transition-colors hover:border-sage-300"
      >
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-sage-100 text-sage-700 dark:bg-sage-900 dark:text-sage-200">
          <Plus size={16} />
        </div>
        <span className="text-[15px] text-muted dark:text-muted-dark">What's taking up space in your mind?</span>
      </button>

      {openWorries && openWorries.length > 0 && (
        <section className="mb-8">
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-muted dark:text-muted-dark">Open worries</h2>
            <button onClick={() => navigate({ name: "worryLoop" })} className="text-xs font-medium text-sage-700 hover:underline dark:text-sage-300">
              View all
            </button>
          </div>
          <div className="flex flex-col gap-2">
            {openWorries.slice(0, 3).map((w) => (
              <Card key={w.id} className="cursor-pointer" onClick={() => navigate({ name: "entry", entryId: w.entryId })}>
                <CardContent className="py-3">
                  <p className="text-sm">{truncate(w.worryText, 120)}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>
      )}

      <section>
        <h2 className="mb-2 text-sm font-semibold text-muted dark:text-muted-dark">Recent entries</h2>
        {entries === null && (
          <div className="flex justify-center py-10 text-muted">
            <Spinner className="h-5 w-5" />
          </div>
        )}
        {entries && entries.length === 0 && (
          <EmptyState
            title="Nothing written yet"
            description="Your first entry doesn't need to be anything in particular. Anchor works as a plain journal from the start, with or without local AI set up."
            action={
              <Button variant="secondary" size="sm" onClick={() => navigate({ name: "newEntry" })}>
                Write your first entry
              </Button>
            }
          />
        )}
        <div className="flex flex-col gap-2">
          {entries?.map((e) => (
            <Card key={e.id} className="cursor-pointer" onClick={() => navigate({ name: "entry", entryId: e.id })}>
              <CardContent className="py-4">
                <div className="mb-1 flex items-center justify-between gap-2">
                  <h3 className="text-sm font-medium">{e.title || "Untitled entry"}</h3>
                  <span className="shrink-0 text-xs text-muted dark:text-muted-dark">{formatDate(e.createdAt)}</span>
                </div>
                <p className="text-sm text-muted dark:text-muted-dark">{truncate(e.body)}</p>
                {e.memoryEnabled && e.indexingStatus !== "ready" && (
                  <Badge tone="default" className="mt-2">
                    {indexingStatusLabel(e.indexingStatus)}
                  </Badge>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      </section>
    </div>
  );
}
