/**
 * Minimal UI primitives — Tailwind styling over Radix accessibility
 * primitives, in the spirit of shadcn/ui (copy-owned components, not an
 * installed component library). Kept in one file for this prototype's
 * scale; a larger app would split these into ui/button.tsx etc.
 */
import * as DialogPrimitive from "@radix-ui/react-dialog";
import * as TabsPrimitive from "@radix-ui/react-tabs";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import * as SelectPrimitive from "@radix-ui/react-select";
import * as ToastPrimitive from "@radix-ui/react-toast";
import React from "react";
import { cn } from "@/lib/utils";
import { Check, ChevronDown, X } from "lucide-react";

// ---------- Button ----------

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md" | "lg";

const buttonVariants: Record<ButtonVariant, string> = {
  primary: "bg-sage-600 text-white hover:bg-sage-700 active:bg-sage-800 disabled:bg-sage-300",
  secondary:
    "bg-surface dark:bg-surface-dark border border-line dark:border-line-dark text-ink dark:text-ink-dark hover:bg-paper dark:hover:bg-black/20",
  ghost: "text-ink dark:text-ink-dark hover:bg-black/5 dark:hover:bg-white/5",
  danger: "bg-rust-500 text-white hover:bg-rust-600",
};
const buttonSizes: Record<ButtonSize, string> = {
  sm: "text-xs px-2.5 py-1.5 rounded-lg gap-1.5",
  md: "text-sm px-3.5 py-2 rounded-xl gap-2",
  lg: "text-base px-5 py-2.5 rounded-xl gap-2",
};

export const Button = React.forwardRef<
  HTMLButtonElement,
  React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: ButtonSize }
>(({ className, variant = "primary", size = "md", ...props }, ref) => (
  <button
    ref={ref}
    className={cn(
      "inline-flex items-center justify-center font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-60",
      buttonVariants[variant],
      buttonSizes[size],
      className
    )}
    {...props}
  />
));
Button.displayName = "Button";

// ---------- Card ----------

export function Card({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-2xl border border-line dark:border-line-dark bg-surface dark:bg-surface-dark shadow-soft",
        className
      )}
      {...props}
    />
  );
}
export function CardHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-5 pt-5", className)} {...props} />;
}
export function CardContent({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-5 py-4", className)} {...props} />;
}
export function CardFooter({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-5 pb-5 pt-1 flex items-center gap-2", className)} {...props} />;
}

// ---------- Form controls ----------

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "w-full rounded-xl border border-line dark:border-line-dark bg-surface dark:bg-surface-dark px-3 py-2 text-sm",
        "placeholder:text-muted dark:placeholder:text-muted-dark focus-visible:ring-0",
        className
      )}
      {...props}
    />
  )
);
Input.displayName = "Input";

export const Textarea = React.forwardRef<HTMLTextAreaElement, React.TextareaHTMLAttributes<HTMLTextAreaElement>>(
  ({ className, ...props }, ref) => (
    <textarea
      ref={ref}
      className={cn(
        "w-full rounded-xl border border-line dark:border-line-dark bg-surface dark:bg-surface-dark px-3 py-2.5 text-[15px] leading-relaxed",
        "placeholder:text-muted dark:placeholder:text-muted-dark resize-y",
        className
      )}
      {...props}
    />
  )
);
Textarea.displayName = "Textarea";

export function Label({ className, ...props }: React.LabelHTMLAttributes<HTMLLabelElement>) {
  return <label className={cn("block text-xs font-medium text-muted dark:text-muted-dark mb-1.5", className)} {...props} />;
}

export function Badge({ className, tone = "default", ...props }: React.HTMLAttributes<HTMLSpanElement> & { tone?: "default" | "sage" | "harbor" | "rust" }) {
  const tones: Record<string, string> = {
    default: "bg-black/5 dark:bg-white/10 text-ink dark:text-ink-dark",
    sage: "bg-sage-100 text-sage-800 dark:bg-sage-900 dark:text-sage-100",
    harbor: "bg-harbor-100 text-harbor-800 dark:bg-harbor-900 dark:text-harbor-100",
    rust: "bg-rust-500/10 text-rust-600 dark:text-rust-500",
  };
  return <span className={cn("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium", tones[tone], className)} {...props} />;
}

// ---------- Switch ----------

export function Switch({ checked, onCheckedChange, disabled }: { checked: boolean; onCheckedChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <SwitchPrimitive.Root
      checked={checked}
      onCheckedChange={onCheckedChange}
      disabled={disabled}
      className={cn(
        "relative h-6 w-10 rounded-full transition-colors data-[state=checked]:bg-sage-600 bg-black/15 dark:bg-white/15",
        "disabled:opacity-50"
      )}
    >
      <SwitchPrimitive.Thumb className="block h-5 w-5 translate-x-0.5 rounded-full bg-white shadow transition-transform data-[state=checked]:translate-x-[18px]" />
    </SwitchPrimitive.Root>
  );
}

// ---------- Tabs ----------

export const Tabs = TabsPrimitive.Root;
export function TabsList({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.List>) {
  return <TabsPrimitive.List className={cn("inline-flex items-center gap-1 rounded-xl bg-black/5 dark:bg-white/5 p-1", className)} {...props} />;
}
export function TabsTrigger({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      className={cn(
        "rounded-lg px-3 py-1.5 text-sm font-medium text-muted dark:text-muted-dark transition-colors",
        "data-[state=active]:bg-surface dark:data-[state=active]:bg-surface-dark data-[state=active]:text-ink dark:data-[state=active]:text-ink-dark data-[state=active]:shadow-soft",
        className
      )}
      {...props}
    />
  );
}
export const TabsContent = TabsPrimitive.Content;

// ---------- Dialog ----------

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export function DialogContent({ className, children, ...props }: React.ComponentProps<typeof DialogPrimitive.Content>) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 bg-black/30 data-[state=open]:animate-in data-[state=open]:fade-in" />
      <DialogPrimitive.Content
        className={cn(
          "fixed left-1/2 top-1/2 w-[90vw] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-2xl bg-surface dark:bg-surface-dark p-6 shadow-soft border border-line dark:border-line-dark",
          className
        )}
        {...props}
      >
        {children}
        <DialogPrimitive.Close className="absolute right-4 top-4 text-muted hover:text-ink dark:hover:text-ink-dark" aria-label="Close">
          <X size={16} />
        </DialogPrimitive.Close>
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}
export const DialogTitle = ({ className, ...props }: React.ComponentProps<typeof DialogPrimitive.Title>) => (
  <DialogPrimitive.Title className={cn("text-base font-semibold mb-2", className)} {...props} />
);
export const DialogDescription = ({ className, ...props }: React.ComponentProps<typeof DialogPrimitive.Description>) => (
  <DialogPrimitive.Description className={cn("text-sm text-muted dark:text-muted-dark mb-4", className)} {...props} />
);

// ---------- Select ----------

export function Select({ value, onValueChange, options, placeholder }: { value?: string; onValueChange: (v: string) => void; options: { value: string; label: string }[]; placeholder?: string }) {
  return (
    <SelectPrimitive.Root value={value} onValueChange={onValueChange}>
      <SelectPrimitive.Trigger className="inline-flex w-full items-center justify-between rounded-xl border border-line dark:border-line-dark bg-surface dark:bg-surface-dark px-3 py-2 text-sm">
        <SelectPrimitive.Value placeholder={placeholder} />
        <SelectPrimitive.Icon>
          <ChevronDown size={14} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content className="overflow-hidden rounded-xl border border-line dark:border-line-dark bg-surface dark:bg-surface-dark shadow-soft z-50">
          <SelectPrimitive.Viewport className="p-1">
            {options.map((o) => (
              <SelectPrimitive.Item
                key={o.value}
                value={o.value}
                className="flex cursor-pointer items-center justify-between rounded-lg px-3 py-2 text-sm outline-none data-[highlighted]:bg-black/5 dark:data-[highlighted]:bg-white/10"
              >
                <SelectPrimitive.ItemText>{o.label}</SelectPrimitive.ItemText>
                <SelectPrimitive.ItemIndicator>
                  <Check size={14} />
                </SelectPrimitive.ItemIndicator>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
}

// ---------- Toast ----------

export const ToastProvider = ToastPrimitive.Provider;
export const ToastViewport = () => (
  <ToastPrimitive.Viewport className="fixed bottom-0 right-0 z-[100] m-0 flex w-full max-w-sm flex-col gap-2 p-4 outline-none" />
);
export function ToastItem({ text, tone, onOpenChange }: { text: string; tone: "default" | "error"; onOpenChange: (open: boolean) => void }) {
  return (
    <ToastPrimitive.Root
      onOpenChange={onOpenChange}
      className={cn(
        "rounded-xl border shadow-soft px-4 py-3 text-sm",
        tone === "error"
          ? "bg-rust-500/10 border-rust-500/30 text-rust-600"
          : "bg-surface dark:bg-surface-dark border-line dark:border-line-dark text-ink dark:text-ink-dark"
      )}
    >
      <ToastPrimitive.Description>{text}</ToastPrimitive.Description>
    </ToastPrimitive.Root>
  );
}

// ---------- Empty / loading states ----------

export function EmptyState({ title, description, action }: { title: string; description?: string; action?: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-line dark:border-line-dark px-8 py-14 text-center">
      <p className="text-sm font-medium text-ink dark:text-ink-dark">{title}</p>
      {description && <p className="max-w-sm text-sm text-muted dark:text-muted-dark">{description}</p>}
      {action}
    </div>
  );
}

export function Spinner({ className }: { className?: string }) {
  return (
    <svg className={cn("animate-spin", className)} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" />
      <path className="opacity-90" d="M4 12a8 8 0 018-8" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}
