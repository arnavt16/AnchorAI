import { useEffect, useState } from "react";
import { ArrowLeft, MessageCircle, Trash2 } from "lucide-react";
import { useAppState } from "@/lib/state";
import { entriesApi, worriesApi, type JournalEntry, type WorryWithHistory } from "@/lib/ipc";
import { Button, Card, CardContent, Input, Textarea, Label, Switch, Badge } from "@/components/ui";
import { formatDateTime, indexingStatusLabel, resultCategoryLabel } from "@/lib/utils";
import { entryFormSchema } from "@/lib/schemas";

const MOODS = ["calm", "anxious", "overwhelmed", "content", "sad", "hopeful", "frustrated", "tired"];

export function EntryScreen({ entryId }: { entryId?: string }) {
  const { navigate, pushToast, refreshSettings } = useAppState();
  const isNew = !entryId;

  const [entry, setEntry] = useState<JournalEntry | null>(null);
  const [worry, setWorry] = useState<WorryWithHistory | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [mood, setMood] = useState<string | undefined>(undefined);
  const [tagsInput, setTagsInput] = useState("");
  const [memoryEnabled, setMemoryEnabled] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [showTrackWorry, setShowTrackWorry] = useState(false);
  const [worryText, setWorryText] = useState("");
  const [expectedOutcome, setExpectedOutcome] = useState("");

  useEffect(() => {
    if (entryId) load(entryId);
  }, [entryId]);

  useEffect(() => {
    function onBeforeUnload(e: BeforeUnloadEvent) {
      if (dirty) {
        e.preventDefault();
        e.returnValue = "";
      }
    }
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [dirty]);

  async function load(id: string) {
    const e = await entriesApi.get(id);
    if (!e) {
      pushToast("That entry couldn't be found.", "error");
      navigate({ name: "home" });
      return;
    }
    setEntry(e);
    setTitle(e.title ?? "");
    setBody(e.body);
    setMood(e.mood ?? undefined);
    setTagsInput(e.tags.join(", "));
    setMemoryEnabled(e.memoryEnabled);
    const worries = await worriesApi.list();
    setWorry(worries.find((w) => w.entryId === id) ?? null);
  }

  function tagsFromInput(): string[] {
    return tagsInput
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean)
      .slice(0, 20);
  }

  async function handleSave() {
    const parsed = entryFormSchema.safeParse({ title: title || undefined, body, mood, tags: tagsFromInput(), memoryEnabled });
    if (!parsed.success) {
      pushToast(parsed.error.issues[0]?.message ?? "Please check the form.", "error");
      return;
    }
    setSaving(true);
    try {
      if (isNew) {
        const created = await entriesApi.save({ title: title || undefined, body, mood, tags: tagsFromInput(), memoryEnabled });
        setDirty(false);
        navigate({ name: "entry", entryId: created.id });
      } else if (entry) {
        const updated = await entriesApi.update(entry.id, { title: title || undefined, body, mood, tags: tagsFromInput() });
        if (memoryEnabled !== entry.memoryEnabled) {
          await entriesApi.setMemoryEligibility(entry.id, memoryEnabled);
        }
        setEntry(updated);
        setDirty(false);
        pushToast("Saved on this device.");
      }
    } catch (err: any) {
      pushToast(err?.message ?? "Couldn't save. Your text is still here — try again.", "error");
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!entry) return;
    if (!confirm("Delete this entry? This also removes its worry history and cannot be undone.")) return;
    await entriesApi.delete(entry.id);
    pushToast("Entry deleted.");
    navigate({ name: "home" });
  }

  async function handleTrackWorry() {
    if (!entry || !worryText.trim()) return;
    const w = await worriesApi.create(entry.id, worryText, expectedOutcome || undefined);
    setWorry({ ...w, outcomes: [], steps: [] });
    setShowTrackWorry(false);
    refreshSettings();
  }

  return (
    <div className="mx-auto max-w-2xl px-8 py-10">
      <div className="mb-6 flex items-center justify-between">
        <button
          onClick={() => navigate({ name: "home" })}
          className="flex items-center gap-1.5 text-sm text-muted hover:text-ink dark:text-muted-dark dark:hover:text-ink-dark"
        >
          <ArrowLeft size={15} /> Journal
        </button>
        {entry && (
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={() => navigate({ name: "reflect", entryId: entry.id })}>
              <MessageCircle size={14} /> Reflect
            </Button>
            <Button variant="ghost" size="sm" onClick={handleDelete}>
              <Trash2 size={14} />
            </Button>
          </div>
        )}
      </div>

      <Input
        value={title}
        onChange={(e) => {
          setTitle(e.target.value);
          setDirty(true);
        }}
        placeholder="Title (optional)"
        className="mb-3 border-none bg-transparent px-0 text-lg font-semibold focus-visible:ring-0"
      />

      <Textarea
        value={body}
        onChange={(e) => {
          setBody(e.target.value);
          setDirty(true);
        }}
        placeholder="What's taking up space in your mind?"
        rows={10}
        className="prose-journal mb-4 border-none bg-transparent px-0 focus-visible:ring-0"
        autoFocus={isNew}
      />

      <div className="mb-6 flex flex-wrap items-center gap-2">
        {MOODS.map((m) => (
          <button
            key={m}
            onClick={() => {
              setMood(mood === m ? undefined : m);
              setDirty(true);
            }}
            className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
              mood === m ? "bg-sage-600 text-white" : "bg-black/5 text-muted hover:bg-black/10 dark:bg-white/5 dark:text-muted-dark"
            }`}
          >
            {m}
          </button>
        ))}
      </div>

      <div className="mb-6">
        <Label htmlFor="tags">Tags</Label>
        <Input
          id="tags"
          value={tagsInput}
          onChange={(e) => {
            setTagsInput(e.target.value);
            setDirty(true);
          }}
          placeholder="comma, separated, tags"
        />
      </div>

      <Card className="mb-6">
        <CardContent className="flex items-start justify-between gap-4 py-4">
          <div>
            <p className="text-sm font-medium">Use in future reflections</p>
            <p className="mt-0.5 text-xs text-muted dark:text-muted-dark">
              Allows this entry to be found later when you reflect, once local AI is set up and you've consented. Off by
              default.
            </p>
            {entry && memoryEnabled && (
              <Badge tone="default" className="mt-2">
                {indexingStatusLabel(entry.indexingStatus)}
              </Badge>
            )}
          </div>
          <Switch
            checked={memoryEnabled}
            onCheckedChange={(v) => {
              setMemoryEnabled(v);
              setDirty(true);
            }}
          />
        </CardContent>
      </Card>

      <div className="mb-8 flex items-center gap-2">
        <Button onClick={handleSave} disabled={saving || !body.trim()}>
          {saving ? "Saving…" : isNew ? "Save entry" : "Save changes"}
        </Button>
        {entry && <span className="text-xs text-muted dark:text-muted-dark">{formatDateTime(entry.updatedAt)}</span>}
      </div>

      {entry && (
        <section className="border-t border-line dark:border-line-dark pt-6">
          <h2 className="mb-3 text-sm font-semibold text-muted dark:text-muted-dark">Worry Loop</h2>
          {!worry && !showTrackWorry && (
            <Button variant="secondary" size="sm" onClick={() => setShowTrackWorry(true)}>
              Track this worry
            </Button>
          )}
          {!worry && showTrackWorry && (
            <Card>
              <CardContent className="flex flex-col gap-3 py-4">
                <div>
                  <Label htmlFor="worry-text">What's the worry, in your own words?</Label>
                  <Textarea id="worry-text" rows={2} value={worryText} onChange={(e) => setWorryText(e.target.value)} />
                </div>
                <div>
                  <Label htmlFor="expected">What do you think might happen? (optional)</Label>
                  <Textarea id="expected" rows={2} value={expectedOutcome} onChange={(e) => setExpectedOutcome(e.target.value)} />
                </div>
                <div className="flex gap-2">
                  <Button size="sm" onClick={handleTrackWorry} disabled={!worryText.trim()}>
                    Save worry
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setShowTrackWorry(false)}>
                    Cancel
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}
          {worry && (
            <Card>
              <CardContent className="py-4">
                <div className="mb-2 flex items-center justify-between">
                  <Badge tone={worry.status === "open" ? "harbor" : "sage"}>{worry.status}</Badge>
                  <button
                    onClick={() => navigate({ name: "worryLoop" })}
                    className="text-xs font-medium text-sage-700 hover:underline dark:text-sage-300"
                  >
                    Open in Worry Loop
                  </button>
                </div>
                <p className="text-sm">{worry.worryText}</p>
                {worry.expectedOutcome && (
                  <p className="mt-1 text-xs text-muted dark:text-muted-dark">Expected: {worry.expectedOutcome}</p>
                )}
                {worry.outcomes.length > 0 && (
                  <div className="mt-3 space-y-2 border-t border-line dark:border-line-dark pt-3">
                    {worry.outcomes.map((o) => (
                      <div key={o.id} className="text-sm">
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-muted dark:text-muted-dark">{formatDateTime(o.recordedAt)}</span>
                          {o.resultCategory && <Badge tone="default">{resultCategoryLabel(o.resultCategory)}</Badge>}
                        </div>
                        <p className="mt-0.5">{o.outcomeText}</p>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          )}
        </section>
      )}
    </div>
  );
}
