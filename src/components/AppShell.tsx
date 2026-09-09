import React from "react";
import { BookOpen, Compass, MessageCircle, Settings as SettingsIcon, Sparkles } from "lucide-react";
import { cn } from "@/lib/utils";
import { useAppState } from "@/lib/state";
import { Badge } from "./ui";

const NAV_ITEMS = [
  { screen: "home" as const, label: "Journal", icon: BookOpen },
  { screen: "worryLoop" as const, label: "Worry Loop", icon: Compass },
  { screen: "reflect" as const, label: "Reflect", icon: MessageCircle },
  { screen: "settings" as const, label: "Settings", icon: SettingsIcon },
];

export function AppShell({ children }: { children: React.ReactNode }) {
  const { screen, navigate, settings, vaultMode } = useAppState();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-paper text-ink dark:bg-paper-dark dark:text-ink-dark">
      <nav className="flex w-[188px] shrink-0 flex-col border-r border-line dark:border-line-dark px-3 py-4" aria-label="Main navigation">
        <div className="mb-6 flex items-center gap-2 px-2">
          <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-sage-600 text-white">
            <Sparkles size={14} />
          </div>
          <span className="text-[15px] font-semibold tracking-tight">Anchor</span>
        </div>

        <div className="flex flex-col gap-1">
          {NAV_ITEMS.map((item) => {
            const active = screen.name === item.screen;
            const Icon = item.icon;
            return (
              <button
                key={item.screen}
                onClick={() => navigate({ name: item.screen } as any)}
                className={cn(
                  "flex items-center gap-2.5 rounded-xl px-3 py-2 text-left text-sm font-medium transition-colors",
                  active
                    ? "bg-sage-100 text-sage-800 dark:bg-sage-900 dark:text-sage-100"
                    : "text-muted hover:bg-black/5 dark:text-muted-dark dark:hover:bg-white/5"
                )}
                aria-current={active ? "page" : undefined}
              >
                <Icon size={16} />
                {item.label}
              </button>
            );
          })}
        </div>

        <div className="mt-auto flex flex-col gap-2 px-2">
          {vaultMode === "demo" && (
            <Badge tone="harbor" className="w-fit">
              Fictional demo vault
            </Badge>
          )}
          <Badge tone={settings?.localAiEnabled ? "sage" : "default"} className="w-fit">
            {settings?.localAiEnabled ? "Local AI on" : "Journal-only mode"}
          </Badge>
        </div>
      </nav>
      <main className="flex-1 overflow-y-auto">{children}</main>
    </div>
  );
}
