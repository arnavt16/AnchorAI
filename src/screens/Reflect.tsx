import { useEffect, useRef, useState } from "react";
import { Send, Library, Save } from "lucide-react";
import { useAppState } from "@/lib/state";
import { reflectApi, type RetrievedSource, type ValidatedReflection } from "@/lib/ipc";
import { Button, Card, CardContent, Textarea, Switch, Badge, Spinner } from "@/components/ui";
import { formatDate, truncate } from "@/lib/utils";

interface Turn {
  role: "user" | "assistant";
  text: string;
  sources?: RetrievedSource[];
  urgent?: boolean;
}

const INTENTIONS: { value: string; label: string }[] = [
  { value: "unpack", label: "Help me unpack this" },
  { value: "related_experience", label: "Look for a related experience" },
  { value: "small_step", label: "Help me find a small next step" },
];

export function ReflectScreen({ seedMessage, entryId }: { seedMessage?: string; entryId?: string }) {
  const { settings, pushToast } = useAppState();
  const [turns, setTurns] = useState<Turn[]>([]);
  const [message, setMessage] = useState(seedMessage ?? "");
  const [intention, setIntention] = useState<string | undefined>(undefined);
  const [useMemory, setUseMemory] = useState(true);
  const [sending, setSending] = useState(false);
  const [drawerSources, setDrawerSources] = useState<RetrievedSource[] | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [turns]);

  const localAiEnabled = settings?.localAiEnabled ?? false;

  async function send() {
    if (!message.trim() || sending) return;
    const userTurn: Turn = { role: "user", text: message };
    setTurns((t) => [...t, userTurn]);
    setSending(true);
    const sentMessage = message;
    setMessage("");
    try {
      const result: ValidatedReflection = await reflectApi.reflect({
        message: sentMessage,
        intention,
        useMemory: useMemory && localAiEnabled,
        excludeEntryId: entryId,
      });
      const text = result.sections.map((s) => s.text).join("\n\n");
      setTurns((t) => [
        ...t,
        { role: "assistant", text: text + (result.followUpQuestion ? `\n\n${result.followUpQuestion}` : ""), sources: result.sources, urgent: result.urgentPathTriggered },
      ]);
    } catch (err: any) {
      setTurns((t) => [...t, { role: "assistant", text: "The local model runtime didn't respond. Your message wasn't lost — set up local AI in Setup, or try again." }]);
    } finally {
      setSending(false);
      setIntention(undefined);
    }
  }

  async function saveAsEntry(text: string) {
    try {
      await reflectApi.saveAsEntry(text, ["reflection"]);
      pushToast("Saved as a new journal entry — you can edit it any time.");
    } catch {
      pushToast("Couldn't save that reflection.", "error");
    }
  }

  return (
    <div className="flex h-full">
      <div className="flex flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-line dark:border-line-dark px-8 py-4">
          <div>
            <h1 className="text-base font-semibold">Reflect</h1>
            <p className="text-xs text-muted dark:text-muted-dark">This conversation isn't saved unless you choose to save part of it.</p>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted dark:text-muted-dark">Use my journal history</span>
            <Switch checked={useMemory} onCheckedChange={setUseMemory} disabled={!localAiEnabled} />
          </div>
        </header>

        {!localAiEnabled && (
          <div className="mx-8 mt-4 rounded-xl border border-line dark:border-line-dark bg-black/[0.02] dark:bg-white/[0.02] px-4 py-3 text-sm text-muted dark:text-muted-dark">
            Local AI isn't set up yet, so reflection will use a general response with no journal history. Set it up any time
            in Settings — this still works as a place to think out loud.
          </div>
        )}

        <div className="flex-1 overflow-y-auto px-8 py-6">
          {turns.length === 0 && (
            <div className="mx-auto max-w-md text-center text-sm text-muted dark:text-muted-dark">
              <p className="mb-4">Start from what's on your mind, or pick an intention below.</p>
            </div>
          )}
          <div className="mx-auto flex max-w-xl flex-col gap-4">
            {turns.map((t, i) => (
              <div key={i} className={t.role === "user" ? "self-end max-w-[85%]" : "self-start max-w-[90%]"}>
                <div
                  className={`rounded-2xl px-4 py-3 text-sm leading-relaxed whitespace-pre-wrap ${
                    t.role === "user"
                      ? "bg-sage-600 text-white"
                      : t.urgent
                      ? "border border-rust-500/40 bg-rust-500/10"
                      : "border border-line dark:border-line-dark bg-surface dark:bg-surface-dark"
                  }`}
                >
                  {t.text}
                </div>
                {t.role === "assistant" && (
                  <div className="mt-1.5 flex items-center gap-3 px-1">
                    {t.sources && t.sources.length > 0 && (
                      <button
                        onClick={() => setDrawerSources(t.sources!)}
                        className="flex items-center gap-1 text-xs font-medium text-sage-700 hover:underline dark:text-sage-300"
                      >
                        <Library size={12} /> {t.sources.length} source{t.sources.length > 1 ? "s" : ""}
                      </button>
                    )}
                    <button
                      onClick={() => saveAsEntry(t.text)}
                      className="flex items-center gap-1 text-xs font-medium text-muted hover:text-ink dark:text-muted-dark dark:hover:text-ink-dark"
                    >
                      <Save size={12} /> Save my reflection
                    </button>
                  </div>
                )}
              </div>
            ))}
            {sending && (
              <div className="self-start">
                <Spinner className="h-4 w-4 text-muted" />
              </div>
            )}
            <div ref={bottomRef} />
          </div>
        </div>

        <div className="border-t border-line dark:border-line-dark px-8 py-4">
          <div className="mx-auto max-w-xl">
            <div className="mb-2 flex flex-wrap gap-1.5">
              {INTENTIONS.map((i) => (
                <button
                  key={i.value}
                  onClick={() => setIntention(intention === i.value ? undefined : i.value)}
                  className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
                    intention === i.value
                      ? "bg-sage-600 text-white"
                      : "bg-black/5 text-muted hover:bg-black/10 dark:bg-white/5 dark:text-muted-dark"
                  }`}
                >
                  {i.label}
                </button>
              ))}
            </div>
            <div className="flex items-end gap-2">
              <Textarea
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    send();
                  }
                }}
                rows={2}
                placeholder="What's on your mind?"
                className="flex-1"
              />
              <Button onClick={send} disabled={!message.trim() || sending} aria-label="Send">
                <Send size={16} />
              </Button>
            </div>
          </div>
        </div>
      </div>

      {drawerSources && (
        <div className="w-[320px] shrink-0 overflow-y-auto border-l border-line dark:border-line-dark px-4 py-6">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold">Sources</h2>
            <button onClick={() => setDrawerSources(null)} className="text-xs text-muted hover:text-ink dark:hover:text-ink-dark">
              Close
            </button>
          </div>
          <div className="flex flex-col gap-3">
            {drawerSources.map((s) => (
              <Card key={s.id}>
                <CardContent className="py-3">
                  <div className="mb-1 flex items-center gap-2">
                    <Badge tone="default">{s.sourceKind}</Badge>
                    <span className="text-xs text-muted dark:text-muted-dark">{formatDate(s.date)}</span>
                  </div>
                  <p className="text-sm">{truncate(s.content, 220)}</p>
                  {s.linkedOutcome && (
                    <div className="mt-2 border-t border-line dark:border-line-dark pt-2 text-sm">
                      <span className="text-xs font-medium text-muted dark:text-muted-dark">Outcome: </span>
                      {truncate(s.linkedOutcome.outcomeText, 160)}
                    </div>
                  )}
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
