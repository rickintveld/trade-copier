import React, { useState, useEffect } from 'react';
import { Worker } from '@/types/trading';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { tradeCopierApi } from '@/lib/api';

interface WorkerFormDialogProps {
  mode: 'create' | 'edit';
  worker?: Worker;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

const WorkerFormDialog: React.FC<WorkerFormDialogProps> = ({
  mode,
  worker,
  open,
  onOpenChange,
  onSuccess,
}) => {
  const [formData, setFormData] = useState({
    name: '',
    port: '5050',
    multiplier: '1.0',
    symbol_prefix: 'none',
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [addressError, setAddressError] = useState('');
  const [submitError, setSubmitError] = useState('');

  // Pre-populate form only when the dialog opens (not on every worker poll update)
  const prevOpenRef = React.useRef(false);
  useEffect(() => {
    const justOpened = open && !prevOpenRef.current;
    prevOpenRef.current = open;

    if (!justOpened) return;

    if (mode === 'edit' && worker) {
      setFormData({
        name: worker.name,
        port: String(worker.port),
        multiplier: String(worker.riskMultiplier),
        symbol_prefix: worker.symbolPrefix || 'none',
      });
    } else if (mode === 'create') {
      setFormData({ name: '', port: '5050', multiplier: '1.0', symbol_prefix: 'none' });
    }

    setSubmitError('');
    setAddressError('');
  }, [open]);

  const validatePort = (port: string): boolean => {
    const portNum = parseInt(port, 10);
    return /^\d+$/.test(port) && portNum > 0 && portNum <= 65535;
  };

  const handlePortChange = (port: string) => {
    setFormData({ ...formData, port });

    if (port && !validatePort(port)) {
      setAddressError('Port must be a number between 1 and 65535');
    } else {
      setAddressError('');
    }
  };

  const handleDialogChange = (nextOpen: boolean) => {
    onOpenChange(nextOpen);
    if (nextOpen) {
      setSubmitError('');
      setAddressError('');
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validatePort(formData.port)) {
      setAddressError('Port must be a number between 1 and 65535');
      return;
    }

    setIsSubmitting(true);
    setSubmitError('');

    try {
      const payload = {
        name: formData.name,
        address: `127.0.0.1:${formData.port}`,
        multiplier: parseFloat(formData.multiplier),
        symbol_prefix: formData.symbol_prefix === 'none' ? '' : formData.symbol_prefix,
      };

      if (mode === 'edit' && worker) {
        await tradeCopierApi.updateWorker(worker.id, payload);
      } else {
        await tradeCopierApi.createWorker(payload);
      }

      onOpenChange(false);
      onSuccess?.();
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error(`Failed to ${mode} worker:`, error);
      }
      const errorMessage = error instanceof Error ? error.message : `Failed to ${mode} worker`;
      setSubmitError(errorMessage);
    } finally {
      setIsSubmitting(false);
    }
  };

  const isEdit = mode === 'edit';

  return (
    <Dialog open={open} onOpenChange={handleDialogChange}>
      <DialogContent onClick={(e) => e.stopPropagation()}>
        <DialogHeader>
          <DialogTitle>{isEdit ? 'Edit Worker' : 'Create New Worker'}</DialogTitle>
          <DialogDescription>
            {isEdit
              ? 'Update the worker settings. The worker will be stopped and restarted if it is currently active.'
              : 'Add a new worker instance to the trade copier system.'}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <label htmlFor="name" className="text-sm font-medium">
                Name
              </label>
              <Input
                id="name"
                placeholder="FTMO-100K"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                required
              />
            </div>
            <div className="grid gap-2">
              <label htmlFor="address" className="text-sm font-medium">
                Port
              </label>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground">
                  127.0.0.1:
                </span>
                <Input
                  id="address"
                  type="number"
                  placeholder="5050"
                  value={formData.port}
                  onChange={(e) => handlePortChange(e.target.value)}
                  className={`pl-20 ${addressError ? 'border-red-500' : ''}`}
                  required
                />
              </div>
              {addressError && (
                <p className="text-sm text-red-500">{addressError}</p>
              )}
            </div>
            <div className="grid gap-2">
              <label htmlFor="multiplier" className="text-sm font-medium">
                Multiplier
              </label>
              <Input
                id="multiplier"
                type="number"
                step="0.1"
                placeholder="e.g. 2.0"
                value={formData.multiplier}
                onChange={(e) => setFormData({ ...formData, multiplier: e.target.value })}
                required
              />
            </div>
            <div className="grid gap-2">
              <label htmlFor="symbol_prefix" className="text-sm font-medium">
                Symbol Prefix
              </label>
              <Select
                value={formData.symbol_prefix}
                onValueChange={(value) => setFormData({ ...formData, symbol_prefix: value })}
              >
                <SelectTrigger id="symbol_prefix">
                  <SelectValue placeholder="None (default)" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">None (default)</SelectItem>
                  <SelectItem value=".ECN">.ECN</SelectItem>
                  <SelectItem value=".RAW">.RAW</SelectItem>
                  <SelectItem value=".PRO">.PRO</SelectItem>
                  <SelectItem value=".ZERO">.ZERO</SelectItem>
                  <SelectItem value=".PRIME">.PRIME</SelectItem>
                  <SelectItem value=".INSTITUTIONAL">.INSTITUTIONAL</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Optional prefix to append to symbol names (e.g., EURUSD → EURUSD.ECN)
              </p>
            </div>
          </div>
          {submitError && (
            <div className="rounded-md bg-red-50 border border-red-200 p-3">
              <p className="text-sm text-red-800">{submitError}</p>
            </div>
          )}
          <DialogFooter>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting
                ? isEdit ? 'Saving...' : 'Creating...'
                : isEdit ? 'Save Changes' : 'Create Worker'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default WorkerFormDialog;
