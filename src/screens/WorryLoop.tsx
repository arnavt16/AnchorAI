import { useEffect, useState } from "react";
import { useAppState } from "@/lib/state";
import { worriesApi, type WorryWithHistory } from "@/lib/ipc";
import { Tabs, TabsList, TabsTrigger, TabsContent, Card, CardContent, Button, Textarea, Label, Select, Badge, EmptyState } from "@/components/ui";
import { formatDate, formatDateTime, resultCategoryLabel } from "@/lib/utils";
import { outcomeFormSchema, stepFormSchema } from "@/lib/schemas";

const RESULT_OPTIONS = [
  { value: "better_than_expected", label: "Better than expected" },
  { value: "about_as_expected", label: "About as expected" },
  { value: "harder_than_expected", label: "Harder than expected" },
  { value: "mixed", label: "Mixed" },
  { value: "still_unsure", label: "Still unsure" },
];

export function WorryLoopScreen() {
  const { pushToast, navigate } = useAppState();
  const [tab, setTab] = useState<"open" | "resolved" | "archived">("open");
  const [worries, setWorries] = useState<WorryWithHistory[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    load();
  }, [tab]);

  async function load() {
    const list = await worriesApi.list(tab);
    setWorries(list);
    if (!list.find((w) => w.id === selected)) setSelected(list[0]?.id ?? null);
  }

  const current = worries?.find((w) => w.id === selected) ?? null;

  return (
    <div className="flex h-full">
      <div className="w-[320px] shrink-0 overflow-y-auto border-r border-line dark:border-line-dark px-4 py-6">
        <h1 className="mb-4 px-1 text-lg font-semibold tracking-tight">Worry Loop</h1>
        <Tabs value={tab} onValueChange={(v) => setTab(v as any)} className="mb-4">
          <TabsList>
            <TabsTrigger value="open">Open</TabsTrigger>
            <TabsTrigger value="resolved">Resolved</TabsTrigger>
            <TabsTrigger value="archived">Archived</TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="flex flex-col gap-2">
          {worries?.length === 0 && <p className="px-1 text-sm text-muted dark:text-muted-dark">Nothing here yet.</p>}
          {worries?.map((w) => (
            <button
              key={w.id}
              onClick={() => setSelected(w.id)}
              className={`rounded-xl border px-3 py-2.5 text-left text-sm transition-colors ${
                selected === w.id
                  ? "border-sage-300 bg-sage-50 dark:border-sage-700 dark:bg-sage-950"
                  : "border-transparent hover:bg-black/5 dark:hover:bg-white/5"
              }`}
            >
              <p className="line-clamp-2">{w.worryText}</p>
              <p className="mt-1 text-xs text-muted dark:text-muted-dark">{formatDate(w.createdAt)}</p>
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-8 py-10">
        {!current && (
          <EmptyState
            title={tab === "open" ? "No open worries" : `No ${tab} worries`}
            description="Track a worry from any journal entry with 'Track this worry.'"
            action={
              <Button variant="secondary" size="sm" onClick={() => navigate({ name: "newEntry" })}>
                Write an entry
              </Button>
            }
          />
        )}
        {current && <WorryDetail worry={current} onChanged={load} onOpenEntry={() => navigate({ name: "entry", entryId: current.entryId })} />}
      </div>
    </div>
  );
}

function WorryDetail({ worry, onChanged, onOpenEntry }: { worry: WorryWithHistory; onChanged: () => void; onOpenEntry: () => void }) {
  const { pushToast } = useAppState();
  const [showOutcomeForm, setShowOutcomeForm] = useState(false);
  const [outcomeText, setOutcomeText] = useState("");
  const [resultCategory, setResultCategory] = useState<string | undefined>(undefined);
  const [reflection, setReflection] = useState("");
  const [showStepForm, setShowStepForm] = useState(false);
  const [stepText, setStepText] = useState("");

  async function submitOutcome() {
    const parsed = outcomeFormSchema.safeParse({ outcomeText, resultCategory, reflection: reflection || undefined });
    if (!parsed.success) {
      pushToast(parsed.error.issues[0]?.message ?? "Please check the form.", "error");
      return;
    }
    await worriesApi.createOutcome(worry.id, outcomeText, resultCategory, reflection || undefined);
    setOutcomeText("");
    setResultCategory(undefined);
    setReflection("");
    setShowOutcomeForm(false);
    onChanged();
    pushToast("Outcome recorded.");
  }

  async function submitStep() {
    const parsed = stepFormSchema.safeParse({ actionText: stepText });
    if (!parsed.success) {
      pushToast(parsed.error.issues[0]?.message ?? "Please check the form.", "error");
      return;
    }
    await worriesApi.createStep(worry.entryId, stepText, worry.id);
    setStepText("");
    setShowStepForm(false);
    onChanged();
  }

  async function setStatus(status: "open" | "resolved" | "archived") {
    await worriesApi.setStatus(worry.id, status);
    onChanged();
  }

  return (
    <div className="mx-auto max-w-xl">
      <div className="mb-4 flex items-center justify-between">
        <Badge tone={worry.status === "open" ? "harbor" : worry.status === "resolved" ? "sage" : "default"}>{worry.status}</Badge>
        <button onClick={onOpenEntry} className="text-xs font-medium text-sage-700 hover:underline dark:text-sage-300">
          View journal entry
        </button>
      </div>

      <h2 className="mb-1 text-lg font-medium leading-snug">{worry.worryText}</h2>
      {worry.expectedOutcome && <p className="mb-4 text-sm text-muted dark:text-muted-dark">Expected: {worry.expectedOutcome}</p>}

      <div className="mb-6 flex gap-2">
        {worry.status !== "resolved" && (
          <Button size="sm" variant="secondary" onClick={() => setStatus("resolved")}>
            Mark resolved
          </Button>
        )}
        {worry.status !== "archived" && (
          <Button size="sm" variant="ghost" onClick={() => setStatus("archived")}>
            Archive
          </Button>
        )}
        {worry.status !== "open" && (
          <Button size="sm" variant="ghost" onClick={() => setStatus("open")}>
            Reopen
          </Button>
        )}
      </div>

      <section className="mb-8">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-muted dark:text-muted-dark">What happened</h3>
          {!showOutcomeForm && (
            <Button size="sm" variant="secondary" onClick={() => setShowOutcomeForm(true)}>
              What happened?
            </Button>
          )}
        </div>

        {showOutcomeForm && (
          <Card className="mb-3">
            <CardContent className="flex flex-col gap-3 py-4">
              <div>
                <Label htmlFor="outcome-text">Outcome</Label>
                <Textarea id="outcome-text" rows={3} value={outcomeText} onChange={(e) => setOutcomeText(e.target.value)} />
              </div>
              <div>
                <Label>Result (optional)</Label>
                <Select value={resultCategory} onValueChange={setResultCategory} options={RESULT_OPTIONS} placeholder="Choose one" />
              </div>
              <div>
                <Label htmlFor="reflection">Reflection (optional)</Label>
                <Textarea id="reflection" rows={2} value={reflection} onChange={(e) => setReflection(e.target.value)} />
              </div>
              <div className="flex gap-2">
                <Button size="sm" onClick={submitOutcome} disabled={!outcomeText.trim()}>
                  Save
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setShowOutcomeForm(false)}>
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        )}

        {worry.outcomes.length === 0 && !showOutcomeForm && (
          <p className="text-sm text-muted dark:text-muted-dark">
            No update yet — that's a valid state. An unresolved worry isn't a problem to fix.
          </p>
        )}
        <div className="flex flex-col gap-3">
          {worry.outcomes.map((o) => (
            <Card key={o.id}>
              <CardContent className="py-3">
                <div className="mb-1 flex items-center gap-2">
                  <span className="text-xs text-muted dark:text-muted-dark">{formatDateTime(o.recordedAt)}</span>
                  {o.resultCategory && <Badge tone="default">{resultCategoryLabel(o.resultCategory)}</Badge>}
                </div>
                <p className="text-sm">{o.outcomeText}</p>
                {o.reflection && <p className="mt-1 text-sm italic text-muted dark:text-muted-dark">{o.reflection}</p>}
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section>
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-muted dark:text-muted-dark">Small steps tried</h3>
          {!showStepForm && (
            <Button size="sm" variant="secondary" onClick={() => setShowStepForm(true)}>
              Add a step
            </Button>
          )}
        </div>
        {showStepForm && (
          <Card className="mb-3">
            <CardContent className="flex flex-col gap-3 py-4">
              <Textarea rows={2} value={stepText} onChange={(e) => setStepText(e.target.value)} placeholder="A small action you tried or plan to try" />
              <div className="flex gap-2">
                <Button size="sm" onClick={submitStep} disabled={!stepText.trim()}>
                  Save
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setShowStepForm(false)}>
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        )}
        <div className="flex flex-col gap-2">
          {worry.steps.map((s) => (
            <StepRow key={s.id} step={s} onChanged={onChanged} />
          ))}
        </div>
      </section>
    </div>
  );
}

function StepRow({ step, onChanged }: { step: WorryWithHistory["steps"][number]; onChanged: () => void }) {
  async function setFeedback(fb: string) {
    await worriesApi.setStepFeedback(step.id, fb);
    onChanged();
  }
  const options: { value: string; label: string }[] = [
    { value: "helped", label: "Helped" },
    { value: "did_not_help", label: "Didn't help" },
    { value: "unsure", label: "Unsure" },
    { value: "not_tried", label: "Haven't tried yet" },
  ];
  return (
    <Card>
      <CardContent className="py-3">
        <p className="mb-2 text-sm">{step.actionText}</p>
        <div className="flex flex-wrap gap-1.5">
          {options.map((o) => (
            <button
              key={o.value}
              onClick={() => setFeedback(o.value)}
              className={`rounded-full px-2.5 py-1 text-xs font-medium transition-colors ${
                step.feedback === o.value
                  ? "bg-sage-600 text-white"
                  : "bg-black/5 text-muted hover:bg-black/10 dark:bg-white/5 dark:text-muted-dark"
              }`}
            >
              {o.label}
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
