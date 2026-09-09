import { useEffect, useState } from "react";
import { AppStateProvider, useAppState } from "@/lib/state";
import { AppShell } from "@/components/AppShell";
import { HomeScreen } from "@/screens/Home";
import { EntryScreen } from "@/screens/Entry";
import { WorryLoopScreen } from "@/screens/WorryLoop";
import { ReflectScreen } from "@/screens/Reflect";
import { SetupScreen } from "@/screens/Setup";
import { SettingsScreen } from "@/screens/Settings";
import { Dialog, DialogContent, DialogTitle, DialogDescription, Button, ToastProvider, ToastViewport, ToastItem } from "@/components/ui";

const ONBOARDING_KEY = "anchor.onboarding.seen.v1";

function Router() {
  const { screen } = useAppState();
  switch (screen.name) {
    case "home":
      return <HomeScreen />;
    case "newEntry":
      return <EntryScreen />;
    case "entry":
      return <EntryScreen entryId={screen.entryId} />;
    case "worryLoop":
      return <WorryLoopScreen />;
    case "reflect":
      return <ReflectScreen seedMessage={screen.seedMessage} entryId={screen.entryId} />;
    case "setup":
      return <SetupScreen />;
    case "settings":
      return <SettingsScreen />;
    default:
      return <HomeScreen />;
  }
}

function Onboarding() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    try {
      if (!localStorage.getItem(ONBOARDING_KEY)) setOpen(true);
    } catch {
      // Private/blocked storage: just don't show a persistent dismissal —
      // showing the dialog every launch is a safe fallback, not a crash.
      setOpen(true);
    }
  }, []);

  function dismiss() {
    try {
      localStorage.setItem(ONBOARDING_KEY, "1");
    } catch {
      // ignore
    }
    setOpen(false);
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && dismiss()}>
      <DialogContent>
        <DialogTitle>A place to write freely</DialogTitle>
        <DialogDescription asChild>
          <div className="space-y-3 text-sm text-ink dark:text-ink-dark">
            <p>Writing is saved on this computer only. Anchor never sends journal content to an external service.</p>
            <p>
              Reflection — connecting a current worry to past ones — is a separate, optional step that runs through
              Ollama, also on this computer. You can write journal-only for as long as you like before setting that up.
            </p>
            <p className="text-xs text-muted dark:text-muted-dark">
              Anchor is a portfolio prototype for reflection and support, not clinically validated treatment or a
              substitute for professional care.
            </p>
          </div>
        </DialogDescription>
        <Button onClick={dismiss} className="mt-2">
          Start journaling
        </Button>
      </DialogContent>
    </Dialog>
  );
}

function ToastHost() {
  const { toasts, dismissToast } = useAppState();
  return (
    <ToastProvider swipeDirection="right">
      {toasts.map((t) => (
        <ToastItem key={t.id} text={t.text} tone={t.tone} onOpenChange={(open) => !open && dismissToast(t.id)} />
      ))}
      <ToastViewport />
    </ToastProvider>
  );
}

export default function App() {
  return (
    <AppStateProvider>
      <AppShell>
        <Router />
      </AppShell>
      <Onboarding />
      <ToastHost />
    </AppStateProvider>
  );
}
