import { useEffect, useState } from "react";
import { save, open, confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import { useAppState } from "@/lib/state";
import { dataApi, entriesApi, systemApi, type JournalEntry } from "@/lib/ipc";
import { Button, Card, CardContent, Switch, Badge, Input } from "@/components/ui";

export function SettingsScreen() {
  const { settings, refreshSettings, vaultMode, refreshVaultMode, pushToast, navigate } = useAppState();
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [search, setSearch] = useState("");
  const [country, setCountry] = useState(settings?.supportCountry ?? "");
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    loadEntries();
  }, [search]);

  useEffect(() => {
    setCountry(settings?.supportCountry ?? "");
  }, [settings?.supportCountry]);

  async function loadEntries() {
    setEntries(await entriesApi.list(search || undefined));
  }

  async function toggleMemory(e: JournalEntry) {
    await entriesApi.setMemoryEligibility(e.id, !e.memoryEnabled);
    loadEntries();
  }

  async function handleExportJson() {
    const json = await dataApi.exportJson();
    const path = await save({ defaultPath: "anchor-export.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    await dataApi.writeTextExport(path as string, json);
    pushToast("Exported to " + path);
  }

  async function handleImportJson() {
    const path = await open({ filters: [{ name: "Anchor export", extensions: ["json"] }] });
    if (!path || Array.isArray(path)) return;
    setBusy("import");
    try {
      const json = await dataApi.readTextImport(path as string);
      const preview = await dataApi.previewImport(json);
      const ok = await confirmDialog(
        `This will add ${preview.entryCount} entr${preview.entryCount === 1 ? "y" : "ies"}, ${preview.worryCount} worr${
          preview.worryCount === 1 ? "y" : "ies"
        }, and ${preview.outcomeCount} outcome${preview.outcomeCount === 1 ? "" : "s"} to the current vault. Imported entries marked for memory won't be indexed until you confirm and rebuild the index.`,
        { title: "Import this file?", kind: "info" }
      );
      if (!ok) return;
      await dataApi.importJson(json);
      pushToast(`Imported ${preview.entryCount} entries.`);
      loadEntries();
    } catch (err: any) {
      pushToast(err?.message ?? "Import failed.", "error");
    } finally {
      setBusy(null);
    }
  }

  async function handleExportMarkdown() {
    const dir = await open({ directory: true });
    if (!dir || Array.isArray(dir)) return;
    setBusy("markdown");
    try {
      const n = await dataApi.exportMarkdown(dir as string);
      pushToast(`Exported ${n} Markdown file${n === 1 ? "" : "s"}.`);
    } finally {
      setBusy(null);
    }
  }

  async function handleBackup() {
    const path = await save({ defaultPath: "anchor-backup.sqlite", filters: [{ name: "Anchor backup", extensions: ["sqlite"] }] });
    if (!path) return;
    setBusy("backup");
    try {
      await dataApi.backup(path);
      pushToast("Backup saved.");
    } finally {
      setBusy(null);
    }
  }

  async function handleRestore() {
    const path = await open({ filters: [{ name: "Anchor backup", extensions: ["sqlite"] }] });
    if (!path || Array.isArray(path)) return;
    const ok = await confirmDialog(
      "This replaces everything currently in this vault with the backup you're restoring. A safety copy of the current vault is made first.",
      { title: "Restore vault?", kind: "warning" }
    );
    if (!ok) return;
    setBusy("restore");
    try {
      await dataApi.restore(path as string);
      pushToast("Vault restored.");
      loadEntries();
      refreshSettings();
    } catch (err: any) {
      pushToast(err?.message ?? "Restore failed.", "error");
    } finally {
      setBusy(null);
    }
  }

  async function handleErase() {
    const ok = await confirmDialog(
      "This permanently erases everything in the current vault. Exports and backups you've made are not affected.",
      { title: "Erase local data?", kind: "warning" }
    );
    if (!ok) return;
    setBusy("erase");
    try {
      await dataApi.erase();
      pushToast("Vault erased.");
      loadEntries();
      refreshSettings();
    } finally {
      setBusy(null);
    }
  }

  async function handleWithdrawConsent() {
    await systemApi.withdrawConsent();
    await refreshSettings();
    pushToast("Local AI consent withdrawn. Indexing has stopped.");
  }

  async function toggleDemoMode() {
    const next = vaultMode === "demo" ? "personal" : "demo";
    await dataApi.switchVaultMode(next);
    if (next === "demo") {
      const n = await dataApi.seedDemoVault();
      if (n > 0) pushToast(`Loaded ${n} fictional demo entries.`);
    }
    await refreshVaultMode();
    await refreshSettings();
    loadEntries();
  }

  async function saveCountry() {
    await systemApi.setSupportCountry(country || undefined);
    await refreshSettings();
    pushToast("Saved.");
  }

  return (
    <div className="mx-auto max-w-2xl px-8 py-10">
      <h1 className="mb-8 text-xl font-semibold tracking-tight">Settings</h1>

      <section className="mb-8">
        <h2 className="mb-3 text-sm font-semibold text-muted dark:text-muted-dark">Demo mode</h2>
        <Card>
          <CardContent className="flex items-center justify-between py-4">
            <div>
              <p className="text-sm font-medium">Fictional demo vault</p>
              <p className="mt-0.5 text-xs text-muted dark:text-muted-dark">
                A separate, isolated vault with made-up entries for trying Anchor out. Never merged with your real
                journal.
              </p>
            </div>
            <Switch checked={vaultMode === "demo"} onCheckedChange={toggleDemoMode} />
          </CardContent>
        </Card>
      </section>

      <section className="mb-8">
        <h2 className="mb-3 text-sm font-semibold text-muted dark:text-muted-dark">Local AI</h2>
        <Card className="mb-3">
          <CardContent className="flex items-center justify-between py-4">
            <div>
              <p className="text-sm font-medium">{settings?.localAiEnabled ? "Local AI is on" : "Local AI is off"}</p>
              <p className="mt-0.5 text-xs text-muted dark:text-muted-dark">
                {settings?.localAiEnabled ? `${settings.chatModel} · ${settings.embeddingModel}` : "Set up in the Setup screen."}
              </p>
            </div>
            <div className="flex gap-2">
              <Button size="sm" variant="secondary" onClick={() => navigate({ name: "setup" })}>
                Setup
              </Button>
              {settings?.localAiEnabled && (
                <Button size="sm" variant="ghost" onClick={handleWithdrawConsent}>
                  Withdraw consent
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
        <Card className="mb-3">
          <CardContent className="flex items-center justify-between py-4">
            <div>
              <p className="text-sm font-medium">Support resources country</p>
              <p className="mt-0.5 text-xs text-muted dark:text-muted-dark">Optional. Used only to show a verified bundled crisis resource if needed.</p>
            </div>
            <div className="flex items-center gap-2">
              <Input value={country} onChange={(e) => setCountry(e.target.value)} placeholder="US, UK, …" className="w-24" />
              <Button size="sm" variant="secondary" onClick={saveCountry}>
                Save
              </Button>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center justify-between py-4">
            <p className="text-sm font-medium">Rebuild index</p>
            <Button size="sm" variant="secondary" onClick={() => systemApi.rebuildIndex().then(() => pushToast("Re-queued eligible entries."))}>
              Rebuild
            </Button>
          </CardContent>
        </Card>
      </section>

      <section className="mb-8">
        <h2 className="mb-3 text-sm font-semibold text-muted dark:text-muted-dark">Memory eligibility</h2>
        <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search entries…" className="mb-3" />
        <div className="flex flex-col gap-2">
          {entries.map((e) => (
            <Card key={e.id}>
              <CardContent className="flex items-center justify-between py-3">
                <div>
                  <p className="text-sm">{e.title || "Untitled entry"}</p>
                  <Badge tone="default" className="mt-1">
                    {e.indexingStatus}
                  </Badge>
                </div>
                <Switch checked={e.memoryEnabled} onCheckedChange={() => toggleMemory(e)} />
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section className="mb-8">
        <h2 className="mb-3 text-sm font-semibold text-muted dark:text-muted-dark">Data</h2>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="secondary" onClick={handleExportJson}>
            Export JSON
          </Button>
          <Button size="sm" variant="secondary" onClick={handleImportJson} disabled={busy === "import"}>
            Import JSON
          </Button>
          <Button size="sm" variant="secondary" onClick={handleExportMarkdown} disabled={busy === "markdown"}>
            Export Markdown
          </Button>
          <Button size="sm" variant="secondary" onClick={handleBackup} disabled={busy === "backup"}>
            Back up vault
          </Button>
          <Button size="sm" variant="secondary" onClick={handleRestore} disabled={busy === "restore"}>
            Restore from backup
          </Button>
          <Button size="sm" variant="danger" onClick={handleErase} disabled={busy === "erase"}>
            Erase local data
          </Button>
        </div>
        <p className="mt-3 text-xs text-muted dark:text-muted-dark">
          Journal storage is plain SQLite in this app's local data folder — not application-encrypted. Your operating
          system's disk encryption, if enabled, is what protects it at rest.
        </p>
      </section>
    </div>
  );
}
