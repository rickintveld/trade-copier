import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DependencyStatus {
  os_name: string;
  os_version: string | null;
  package_manager_name: string;
  package_manager_status: "pending" | "installing" | "installed" | "error";
  wine_status: "pending" | "installing" | "installed" | "error";
  error_message: string | null;
  all_installed: boolean;
}

export function useDependencyStatus() {
  const [status, setStatus] = useState<DependencyStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = async () => {
    try {
      setLoading(true);
      const response = await invoke<{ success: boolean; data: DependencyStatus | null }>(
        "get_dependency_status"
      );
      
      if (response.success && response.data) {
        setStatus(response.data);
      } else {
        // No status yet, trigger check
        await checkDependencies();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const checkDependencies = async () => {
    try {
      setLoading(true);
      const response = await invoke<{ success: boolean; data: DependencyStatus }>(
        "check_dependencies"
      );
      
      if (response.success) {
        setStatus(response.data);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const installDependencies = async () => {
    try {
      setError(null);
      const response = await invoke<{ success: boolean; data: DependencyStatus }>(
        "install_dependencies"
      );
      
      if (response.success) {
        setStatus(response.data);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  useEffect(() => {
    fetchStatus();

    // Listen for dependency status changes
    const unlisten = listen<DependencyStatus>("dependency-status-changed", (event) => {
      setStatus(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return {
    status,
    loading,
    error,
    checkDependencies,
    installDependencies,
  };
}
