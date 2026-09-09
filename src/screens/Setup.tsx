import { useEffect, useState } from "react";
import { CheckCircle2, XCircle, RefreshCw } from "lucide-react";
import { useAppState } from "@/lib/state";
import { systemApi, type RuntimeStatus, type ReadinessResult } from "@/lib/ipc";
import { Button, Card, CardContent, Select, Badge, Spinner } from "@/components/ui";

// A short, deliberately small allowlist of models known to work well with
// Anchor's reflection pipeline at prototype scale — a mix of strong
// structured-JSON compliance (for the cited path) and warm conversational
// tone (for the natural-language fallback path), since Reflect now uses
// both. This is NOT an exhaustive list of everything Ollama can run — see
// README for how to point Anchor at a different model you've verified
// yourself.
const SUGGESTED_CHAT_MODELS = ["qwen3.5:9b", "llama3.1:8b", "gemma4:12b"];
const SUGGESTED_EMBEDDING_MODELS = ["nomic-embed-text", "mxbai-embed-large", "all-minilm"];

export function SetupScreen() {
  const { settings, refreshSettings, pushToast } = useAppState();
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [chatModel, setChatModel] = useState<string | undefined>(undefined);
  const [embeddingModel, setEmbeddingModel] = useState<string | undefined>(undefined);
  const [readiness, setReadiness] = useState<ReadinessResult | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    checkRuntime();
  }, []);

  async function checkRuntime() {
    setChecking(true);
    try {
      const r = await systemApi.checkRuntime();
      setRuntime(r);
    } catch {
      setRuntime({ reachable: false, localModels: [] });
    } finally {
      setChecking(false);
    }
  }

  const chatOptions = (runtime?.localModels.length ? runtime.localModels : SUGGESTED_CHAT_MODELS).map((m) => ({ value: m, label: m }));
  const embeddingOptions = (runtime?.localModels.length ? runtime.localModels : SUGGESTED_EMBEDDING_MODELS).map((m) => ({ value: m, label: m }));

  async function grantAndRun() {
    if (!chatModel || !embeddingModel) return;
    setRunning(true);
    try {
      if (!settings?.localAiEnabled) {
        await systemApi.grantConsent();
      }
      const result = await systemApi.runReadinessCheck(chatModel, embeddingModel);
      setReadiness(result);
      if (result.embeddingOk && result.chatOk) {
        pushToast("Local AI is ready.");
      }
      await refreshSettings();
    } catch (err: any) {
      pushToast(err?.message ?? "Readiness check failed.", "error");
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="mx-auto max-w-xl px-8 py-10">
      <h1 className="mb-1 text-xl font-semibold tracking-tight">Set up local AI</h1>
      <p className="mb-8 text-sm text-muted dark:text-muted-dark">
        Anchor's journal works without this. This step turns on reflection — semantic search and chat, both running on
        this computer through Ollama. Nothing here is required to keep writing.
      </p>

      <Card className="mb-6">
        <CardContent className="py-4">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold">1. Ollama runtime</h2>
            <Button size="sm" variant="ghost" onClick={checkRuntime} disabled={checking}>
              <RefreshCw size={13} className={checking ? "animate-spin" : ""} />
            </Button>
          </div>
          {checking && <Spinner className="h-4 w-4 text-muted" />}
          {!checking && runtime && (
            <div className="flex items-center gap-2 text-sm">
              {runtime.reachable ? <CheckCircle2 size={16} className="text-sage-600" /> : <XCircle size={16} className="text-rust-500" />}
              {runtime.reachable ? (
                <span>
                  Reachable at 127.0.0.1:11434 — {runtime.localModels.length} local model{runtime.localModels.length === 1 ? "" : "s"} found.
                </span>
              ) : (
                <span>Not reachable. Install Ollama from ollama.com and make sure it's running, then check again.</span>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="mb-6">
        <CardContent className="flex flex-col gap-4 py-4">
          <h2 className="text-sm font-semibold">2. Choose models</h2>
          <p className="text-xs text-muted dark:text-muted-dark">
            Pull these yourself with <code className="rounded bg-black/5 px-1 dark:bg-white/10">ollama pull &lt;model&gt;</code> first if they aren't
            listed. Anchor does not download models on your behalf in this version — see README for why.
          </p>
          <div>
            <p className="mb-1.5 text-xs font-medium text-muted dark:text-muted-dark">Chat model</p>
            <Select value={chatModel} onValueChange={setChatModel} options={chatOptions} placeholder="Choose a chat model" />
          </div>
          <div>
            <p className="mb-1.5 text-xs font-medium text-muted dark:text-muted-dark">Embedding model</p>
            <Select value={embeddingModel} onValueChange={setEmbeddingModel} options={embeddingOptions} placeholder="Choose an embedding model" />
          </div>
        </CardContent>
      </Card>

      <Card className="mb-6">
        <CardContent className="flex flex-col gap-3 py-4">
          <h2 className="text-sm font-semibold">3. Consent &amp; readiness check</h2>
          <p className="text-xs text-muted dark:text-muted-dark">
            This runs one embedding and one short chat request using synthetic test text only — never your journal
            content — and checks the results are well-formed before turning reflection on.
          </p>
          <Button onClick={grantAndRun} disabled={!chatModel || !embeddingModel || running}>
            {running ? "Checking…" : "Run readiness check"}
          </Button>
          {readiness && (
            <div className="mt-2 flex flex-col gap-1.5 text-sm">
              <div className="flex items-center gap-2">
                {readiness.embeddingOk ? <CheckCircle2 size={15} className="text-sage-600" /> : <XCircle size={15} className="text-rust-500" />}
                Embedding: {readiness.embeddingOk ? `${readiness.embeddingDimension} dimensions, finite values` : readiness.embeddingError}
              </div>
              <div className="flex items-center gap-2">
                {readiness.chatOk ? <CheckCircle2 size={15} className="text-sage-600" /> : <XCircle size={15} className="text-rust-500" />}
                Chat: {readiness.chatOk ? "responded to a structured test prompt" : readiness.chatError}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {settings?.localAiEnabled && (
        <p className="text-sm text-sage-700 dark:text-sage-300">
          Local AI is on. Chat model: {settings.chatModel ?? "—"} · Embedding model: {settings.embeddingModel ?? "—"}
        </p>
      )}
    </div>
  );
}
