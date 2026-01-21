import { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { useDependencyStatus } from "@/hooks/useDependencyStatus";
import { CheckCircle2, ExternalLink, AlertCircle, CheckCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { stat } from "fs";

const SetupStep = ({ 
  number, 
  title, 
  description,
  isWarning = false,
  isReady = false,
}: { 
  number: number; 
  title: string; 
  description: string;
  isWarning?: boolean;
  isReady?: boolean;
}) => {
  return (
    <div className="flex gap-3 p-3 border rounded-lg">
      <div className="flex-shrink-0">
        <div className="flex items-center justify-center w-8 h-8 rounded-full bg-primary text-primary-foreground font-semibold text-sm">
          {number}
        </div>
      </div>
      <div className="flex-1 space-y-1">
        <h4 className="font-medium text-sm">{title}</h4>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {isWarning && (
        <AlertCircle className="h-5 w-5 text-yellow-500 flex-shrink-0" />
      )}
      {isReady && (
        <CheckCircle className="h-5 w-5 text-green-500 flex-shrink-0" />
      )}
    </div>
  );
};

interface DependencyCheckModalProps {
  isOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function DependencyCheckModal({ isOpen, onOpenChange }: DependencyCheckModalProps = {}) {
  const { status } = useDependencyStatus();
  const [internalOpen, setInternalOpen] = useState(true);

  // Use external control if provided, otherwise use internal state
  const open = isOpen !== undefined ? isOpen : internalOpen;
  const setOpen = onOpenChange || setInternalOpen;

  useEffect(() => {
    // Only auto-show on first load when not externally controlled
    if (isOpen === undefined) {
      const hasSeenSetup = localStorage.getItem('hasSeenSetupGuide');
      if (hasSeenSetup) {
        setInternalOpen(false);
      }
    }
  }, [isOpen]);

  const handleClose = () => {
    setOpen(false);
    if (isOpen === undefined) {
      localStorage.setItem('hasSeenSetupGuide', 'true');
    }
  };

  const videoUrl = "https://www.youtube.com/watch?v=t42wSa0e-jQ";

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Setup Guide</DialogTitle>
          <DialogDescription>
            Follow these steps to get Trade Copier working correctly
          </DialogDescription>
        </DialogHeader>

        {/* Video Link */}
        <div className="space-y-3">
          <a 
            href={videoUrl}
            target="_blank" 
            rel="noopener noreferrer"
            className="flex items-center justify-center gap-2 p-4 border-2 border-primary rounded-lg hover:bg-primary/10 transition-colors"
          >
            <span className="font-semibold">Watch Setup Video Tutorial</span>
            <ExternalLink className="h-4 w-4" />
          </a>

          {/* Warning Alert */}
          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>
              MetaTrader 5 must be installed for the Expert Advisor copying functionality to work. If not already installed, install MT5 and restart this app.
            </AlertDescription>
          </Alert>

          {/* Setup Steps */}
          <div className="space-y-3 mt-4">
            <h3 className="font-semibold text-sm mb-2">Quick Setup Steps:</h3>
            
            <SetupStep
              number={1}
              title="Install MetaTrader 5"
              description="Download and install MetaTrader 5 from your broker or MetaQuotes.net. After installation, restart this app."
              isWarning={!status?.all_installed}
              isReady={status?.all_installed}
            />

            <SetupStep
              number={2}
              title="Configure MT5 Settings (Do this on ALL terminals)"
              description="In MT5: Tools → Options → Expert Advisors tab. Check ✓ 'Allow DLL imports' and ✓ 'Allow WebRequest for listed URLs'. Add http://127.0.0.1 to the URL list. Click OK."
              isWarning={true}
            />

            <SetupStep
              number={3}
              title="Setup Master Terminal"
              description="Expert Advisors are automatically installed to your master MT5. Open MT5 Navigator (Ctrl+N), expand Expert Advisors → Trading Rocket. Drag 'Signal Provider' onto any chart. Keep default settings (RouterIP: 127.0.0.1, RouterPort: 5000). Enable AutoTrading (Ctrl+E)."
            />

            <SetupStep
              number={4}
              title="Create Worker Instance"
              description="In this dashboard, create a worker instance with your settings (name, address like 127.0.0.1:5050, and lot multiplier). The app will automatically create a slave MT5 terminal with the Signal Receiver EA already installed. Start the worker."
            />

            <SetupStep
              number={5}
              title="Setup Slave Terminal"
              description="Open the slave MT5 terminal (created automatically). Navigate to Expert Advisors → Trading Rocket. Drag 'Signal Receiver' onto any chart. Set WorkerIP: 127.0.0.1 and WorkerPort: 5050 (match your worker). Enable AutoTrading (Ctrl+E)."
            />

            <SetupStep
              number={6}
              title="Verify Everything Works"
              description="Check MT5 Experts tab (Ctrl+T) for '[SENDER] Connected to router' and '[RECEIVER] Connected to worker' messages. In dashboard, verify worker shows 'mt5_connected: true'. Place a test trade in master terminal."
            />
          </div>

          {/* Success indicator if MT5 detected */}
          {status?.all_installed && (
            <div className="flex items-center gap-2 p-3 bg-green-50 border border-green-200 rounded-lg">
              <CheckCircle2 className="h-5 w-5 text-green-600" />
              <span className="text-sm font-medium text-green-700">
                System dependencies are installed and ready!
              </span>
            </div>
          )}
        </div>

        <Button onClick={handleClose} className="w-full mt-4">
          Got it, Let's Start
        </Button>
      </DialogContent>
    </Dialog>
  );
}
