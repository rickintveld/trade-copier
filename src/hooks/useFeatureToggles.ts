import { useState, useEffect, useCallback } from 'react';
import { tradeCopierApi, ApiFeatureToggle } from '@/lib/api';

interface UseFeatureTogglesReturn {
  toggles: ApiFeatureToggle[];
  isEnabled: (key: string) => boolean;
  toggle: (key: string, enabled: boolean) => Promise<void>;
  loading: boolean;
}

export const useFeatureToggles = (): UseFeatureTogglesReturn => {
  const [toggles, setToggles] = useState<ApiFeatureToggle[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchToggles = useCallback(async () => {
    try {
      const data = await tradeCopierApi.getFeatureToggles();
      setToggles(data);
    } catch (error) {
      if (import.meta.env.DEV) {
        console.warn('Failed to fetch feature toggles:', error);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchToggles();
  }, [fetchToggles]);

  const isEnabled = useCallback(
    (key: string): boolean => {
      const found = toggles.find((t) => t.key === key);
      return found ? found.enabled : true; // default to enabled if not found
    },
    [toggles],
  );

  const toggle = useCallback(
    async (key: string, enabled: boolean) => {
      // Optimistic update
      setToggles((prev) =>
        prev.map((t) => (t.key === key ? { ...t, enabled } : t)),
      );

      try {
        await tradeCopierApi.updateFeatureToggle(key, enabled);
      } catch (error) {
        // Revert on failure
        setToggles((prev) =>
          prev.map((t) => (t.key === key ? { ...t, enabled: !enabled } : t)),
        );
        throw error;
      }
    },
    [],
  );

  return { toggles, isEnabled, toggle, loading };
};
