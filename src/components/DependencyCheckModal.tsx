import { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { useDependencyStatus, DependencyStatus } from "@/hooks/useDependencyStatus";
import { Loader2, CheckCircle2, XCircle, AlertCircle, Monitor, Package, Wine } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";

const StatusIcon = ({ status }: { status: string }) => {
  switch (status) {
    case "installed":
      return <CheckCircle2 className="h-5 w-5 text-green-500" />;
    case "installing":
      return <Loader2 className="h-5 w-5 text-blue-500 animate-spin" />;
    case "error":
      return <XCircle className="h-5 w-5 text-red-500" />;
    case "pending":
    default:
      return <AlertCircle className="h-5 w-5 text-yellow-500" />;
  }
};

const StatusText = ({ status }: { status: string }) => {
  switch (status) {
    case "installed":
      return <span className="text-green-600 font-medium">Installed</span>;
    case "installing":
      return <span className="text-blue-600 font-medium">Installing...</span>;
    case "error":
      return <span className="text-red-600 font-medium">Error</span>;
    case "pending":
    default:
      return <span className="text-yellow-600 font-medium">Pending</span>;
  }
};

const DependencyItem = ({ 
  icon: Icon, 
  label, 
  status 
}: { 
  icon: any; 
  label: string; 
  status: string;
}) => {
  return (
    <div className="flex items-center justify-between p-4 border rounded-lg">
      <div className="flex items-center gap-3">
        <Icon className="h-6 w-6 text-gray-600" />
        <span className="font-medium text-white-600">{label}</span>
      </div>
      <div className="flex items-center gap-2">
        <StatusIcon status={status} />
        <StatusText status={status} />
      </div>
    </div>
  );
};

export function DependencyCheckModal() {
  const { status, loading, error, installDependencies } = useDependencyStatus();
  const [open, setOpen] = useState(false);
  const [hasStartedInstall, setHasStartedInstall] = useState(false);

  useEffect(() => {
    // Show modal if dependencies are not all installed
    if (status && !status.all_installed) {
      setOpen(true);
      
      // Auto-start installation if not started yet
      if (!hasStartedInstall && 
          (status.package_manager_status === "pending" || status.wine_status === "pending")) {
        setHasStartedInstall(true);
        installDependencies();
      }
    } else if (status && status.all_installed) {
      // Auto-close when all dependencies are installed
      setOpen(false);
    }
  }, [status, hasStartedInstall]);

  if (loading && !status) {
    return null;
  }

  if (!status) {
    return null;
  }

  const needsInstallation = !status.all_installed;

  return (
    <Dialog open={open} onOpenChange={() => {}}>
      <DialogContent 
        className="sm:max-w-md"
        onPointerDownOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>System Dependencies</DialogTitle>
          <DialogDescription>
            Setting up required dependencies for Trade Copier
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          {/* OS Information */}
          <DependencyItem
            icon={Monitor}
            label={`${status.os_name}${status.os_version ? ` ${status.os_version}` : ""}`}
            status="installed"
          />

          {/* Package Manager */}
          <DependencyItem
            icon={Package}
            label={status.package_manager_name}
            status={status.package_manager_status}
          />

          {/* Wine (only on macOS/Linux) */}
          {status.os_name !== "Windows" && (
            <DependencyItem
              icon={Wine}
              label="Wine"
              status={status.wine_status}
            />
          )}
        </div>

        {/* Error Message */}
        {error && (
          <Alert variant="destructive">
            <XCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {status.error_message && (
          <Alert variant="destructive">
            <XCircle className="h-4 w-4" />
            <AlertDescription>{status.error_message}</AlertDescription>
          </Alert>
        )}

        {/* Retry Button (only show on error) */}
        {(status.package_manager_status === "error" || status.wine_status === "error") && (
          <Button 
            onClick={() => {
              setHasStartedInstall(true);
              installDependencies();
            }}
            className="w-full"
          >
            Retry Installation
          </Button>
        )}

        {/* Installing message */}
        {(status.package_manager_status === "installing" || status.wine_status === "installing") && (
          <p className="text-sm text-center text-white-600">
            Please wait while dependencies are being installed. This may take several minutes.
          </p>
        )}

        {/* Success message */}
        {status.all_installed && (
          <p className="text-sm text-center text-green-600 font-medium">
            All dependencies installed successfully!
          </p>
        )}
      </DialogContent>
    </Dialog>
  );
}
