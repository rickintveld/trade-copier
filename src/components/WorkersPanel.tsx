import React, { useState } from 'react';
import { Worker } from '@/types/trading';
import WorkerCard from './WorkerCard';
import { Users, ChevronLeft, ChevronRight, Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { tradeCopierApi } from '@/lib/api';

interface WorkersPanelProps {
  workers: Worker[];
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  onWorkerCreated?: () => void;
}

const WorkersPanel: React.FC<WorkersPanelProps> = ({ 
  workers, 
  isCollapsed, 
  onToggleCollapse,
  onWorkerCreated
}) => {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [formData, setFormData] = useState({
    name: '',
    address: '',
    multiplier: '1.0'
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [addressError, setAddressError] = useState('');
  const [submitError, setSubmitError] = useState('');

  const activeCount = workers.filter(w => w.status === 'active').length;
  const errorCount = workers.filter(w => w.status === 'error').length;

  const validateAddress = (address: string): boolean => {
    const pattern = /^0\.0\.0\.0:\d+$/;
    return pattern.test(address);
  };

  const handleAddressChange = (address: string) => {
    setFormData({ ...formData, address });
    
    if (address && !validateAddress(address)) {
      setAddressError('Address must be in format 0.0.0.0:port (e.g. 0.0.0.0:5050)');
    } else {
      setAddressError('');
    }
  };

  const handleDialogChange = (open: boolean) => {
    setIsDialogOpen(open);
    if (open) {
      setSubmitError('');
      setAddressError('');
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateAddress(formData.address)) {
      setAddressError('Address must be in format 0.0.0.0:port (e.g. 0.0.0.0:5050)');
      return;
    }
    
    setIsSubmitting(true);
    setSubmitError('');
    
    try {
      await tradeCopierApi.createWorker({
        name: formData.name,
        address: formData.address,
        multiplier: parseFloat(formData.multiplier)
      });
      
      setIsDialogOpen(false);
      setFormData({ name: '', address: '', multiplier: '1.0' });
      setAddressError('');
      
      if (onWorkerCreated) {
        onWorkerCreated();
      }
    } catch (error) {
      console.error('Failed to create worker:', error);
      const errorMessage = error instanceof Error ? error.message : 'Failed to create worker';
      setSubmitError(errorMessage);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className={`relative flex flex-col bg-sidebar border-r border-border/50 transition-all duration-300 ${
        isCollapsed ? 'w-16' : 'w-80'
      }`}
    >
      <div className="flex items-center justify-between p-4 border-b border-border/50">
        {!isCollapsed && (
          <div className="flex items-center gap-2">
            <Users className="w-5 h-5 text-primary" />
            <h2 className="font-semibold text-foreground">Workers</h2>
          </div>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={onToggleCollapse}
          className="h-8 w-8 text-muted-foreground hover:text-foreground"
        >
          {isCollapsed ? (
            <ChevronRight className="w-4 h-4" />
          ) : (
            <ChevronLeft className="w-4 h-4" />
          )}
        </Button>
      </div>

      {!isCollapsed && (
        <>
          <div className="flex items-center gap-4 px-4 py-3 bg-muted/30">
            <div className="flex items-center gap-1.5">
              <div className="w-2 h-2 rounded-full bg-status-active" />
              <span className="text-xs text-muted-foreground">
                {activeCount} Active
              </span>
            </div>
            {errorCount > 0 && (
              <div className="flex items-center gap-1.5">
                <div className="w-2 h-2 rounded-full bg-status-error" />
                <span className="text-xs text-muted-foreground">
                  {errorCount} Error
                </span>
              </div>
            )}
          </div>

          <div className="px-3 py-2">
            <Dialog open={isDialogOpen} onOpenChange={handleDialogChange}>
              <DialogTrigger asChild>
                <Button className="w-full" variant="outline" size="sm">
                  <Plus className="w-4 h-4 mr-2" />
                  Add Worker
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Create New Worker</DialogTitle>
                  <DialogDescription>
                    Add a new worker instance to the trade copier system.
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
                        Address
                      </label>
                      <Input
                        id="address"
                        placeholder="e.g. 0.0.0.0:5050"
                        value={formData.address}
                        onChange={(e) => handleAddressChange(e.target.value)}
                        className={addressError ? 'border-red-500' : ''}
                        required
                      />
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
                  </div>
                  {submitError && (
                    <div className="rounded-md bg-red-50 border border-red-200 p-3">
                      <p className="text-sm text-red-800">{submitError}</p>
                    </div>
                  )}
                  <DialogFooter>
                    <Button type="submit" disabled={isSubmitting}>
                      {isSubmitting ? 'Creating...' : 'Create Worker'}
                    </Button>
                  </DialogFooter>
                </form>
              </DialogContent>
            </Dialog>
          </div>

          <div className="flex-1 overflow-y-auto scrollbar-thin p-3 space-y-3">
            {workers.map((worker) => (
              <WorkerCard key={worker.id} worker={worker} />
            ))}
          </div>
        </>
      )}

      {isCollapsed && (
        <div className="flex-1 overflow-y-auto p-2 space-y-2">
          {workers.map((worker) => (
            <div
              key={worker.id}
              className="relative group"
              title={`${worker.name} - ${worker.status}`}
            >
              <div
                className={`w-10 h-10 rounded-lg flex items-center justify-center text-xs font-semibold cursor-pointer transition-all ${
                  worker.status === 'active'
                    ? 'bg-status-active/20 text-status-active border border-status-active/30'
                    : worker.status === 'error'
                    ? 'bg-status-error/20 text-status-error border border-status-error/30'
                    : 'bg-status-inactive/20 text-status-inactive border border-status-inactive/30'
                }`}
              >
                {worker.name.slice(-2)}
              </div>
              {worker.status === 'active' && (
                <div className="absolute top-0 right-0 w-2 h-2 rounded-full bg-status-active animate-pulse" />
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default WorkersPanel;
