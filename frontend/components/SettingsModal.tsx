import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { useFeatureToggles } from "@/hooks/useFeatureToggles";
import { CalendarDays, AlertCircle, Loader2 } from "lucide-react";

const FEATURE_META: Record<string, { label: string; description: string; icon: React.ElementType }> = {
  news_notifications: {
    label: "News Notifications",
    description: "Show toast alerts for upcoming high-impact economic events and trading restrictions.",
    icon: CalendarDays,
  },
  error_notifications: {
    label: "Error Notifications",
    description: "Show toast alerts when critical worker errors occur.",
    icon: AlertCircle,
  },
};

interface SettingsModalProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsModal({ isOpen, onOpenChange }: SettingsModalProps) {
  const { toggles, isEnabled, toggle, loading } = useFeatureToggles();

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>
            Configure notifications and feature preferences.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-1 mt-2">
          <h4 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">Notifications</h4>

          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <div className="space-y-3 mt-3">
              {toggles.map((ft) => {
                const meta = FEATURE_META[ft.key];
                if (!meta) return null;
                const Icon = meta.icon;

                return (
                  <div
                    key={ft.key}
                    className="flex items-center justify-between gap-4 p-3 rounded-lg border border-border/50 bg-card/50"
                  >
                    <div className="flex items-start gap-3">
                      <Icon className="h-5 w-5 text-muted-foreground mt-0.5 shrink-0" />
                      <div>
                        <p className="text-sm font-medium">{meta.label}</p>
                        <p className="text-xs text-muted-foreground">{meta.description}</p>
                      </div>
                    </div>
                    <Switch
                      checked={isEnabled(ft.key)}
                      onCheckedChange={(checked) => toggle(ft.key, checked)}
                    />
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
