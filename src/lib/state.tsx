import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { systemApi, dataApi, type AppSettings, type VaultMode } from "./ipc";

export type Screen =
  | { name: "home" }
  | { name: "entry"; entryId: string }
  | { name: "newEntry" }
  | { name: "worryLoop" }
  | { name: "reflect"; seedMessage?: string; entryId?: string }
  | { name: "setup" }
  | { name: "settings" };

export interface Toast {
  id: string;
  text: string;
  tone: "default" | "error";
}

interface AppStateShape {
  settings: AppSettings | null;
  refreshSettings: () => Promise<void>;
  vaultMode: VaultMode;
  refreshVaultMode: () => Promise<void>;
  screen: Screen;
  navigate: (s: Screen) => void;
  toasts: Toast[];
  pushToast: (text: string, tone?: Toast["tone"]) => void;
  dismissToast: (id: string) => void;
}

const AppStateContext = createContext<AppStateShape | null>(null);

export function AppStateProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [vaultMode, setVaultMode] = useState<VaultMode>("personal");
  const [screen, setScreen] = useState<Screen>({ name: "home" });
  const [toasts, setToasts] = useState<Toast[]>([]);

  const refreshSettings = useCallback(async () => {
    try {
      setSettings(await systemApi.getSettings());
    } catch {
      // Settings failing to load shouldn't crash the whole app; screens
      // that depend on it (Setup, Settings) handle `null` explicitly.
    }
  }, []);

  const refreshVaultMode = useCallback(async () => {
    try {
      setVaultMode(await dataApi.getVaultMode());
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    refreshSettings();
    refreshVaultMode();
  }, [refreshSettings, refreshVaultMode]);

  const pushToast = useCallback((text: string, tone: Toast["tone"] = "default") => {
    const id = Math.random().toString(36).slice(2);
    setToasts((t) => [...t, { id, text, tone }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts((t) => t.filter((x) => x.id !== id));
  }, []);

  const value = useMemo<AppStateShape>(
    () => ({
      settings,
      refreshSettings,
      vaultMode,
      refreshVaultMode,
      screen,
      navigate: setScreen,
      toasts,
      pushToast,
      dismissToast,
    }),
    [settings, refreshSettings, vaultMode, refreshVaultMode, screen, toasts, pushToast, dismissToast]
  );

  return <AppStateContext.Provider value={value}>{children}</AppStateContext.Provider>;
}

export function useAppState() {
  const ctx = useContext(AppStateContext);
  if (!ctx) throw new Error("useAppState must be used within AppStateProvider");
  return ctx;
}
