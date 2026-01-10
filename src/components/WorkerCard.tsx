import React, { useState } from 'react';
import { Worker } from '@/types/trading';
import { formatRelativeTime, formatLatency } from '@/lib/formatters';
import { tradeCopierApi } from '@/lib/api';
import { Wifi, WifiOff, Activity, Clock, Gauge, Trash2, RotateCw } from 'lucide-react';

interface WorkerCardProps {
  worker: Worker;
}

const WorkerCard: React.FC<WorkerCardProps> = ({ worker }) => {
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleStart = async (force: boolean = false) => {
    try {
      await tradeCopierApi.startWorker(worker.id, force);
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to start worker:', error);
      }
    }
  };

  const handleStop = async () => {
    try {
      await tradeCopierApi.stopWorker(worker.id);
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to stop worker:', error);
      }
    }
  };

  const handleDelete = async () => {
    try {
      await tradeCopierApi.deleteWorker(worker.id);
      setShowDeleteConfirm(false);
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to delete worker:', error);
      }
    }
  };
  const statusColors = {
    active: 'border-l-status-active bg-status-active/5',
    inactive: 'border-l-status-inactive bg-status-inactive/5',
    installing: 'border-l-status-warning bg-status-warning/5',
    error: 'border-l-status-error bg-status-error/5',
  };

  const statusIndicatorColors = {
    active: 'bg-status-active',
    inactive: 'bg-status-inactive',
    installing: 'bg-status-warning',
    error: 'bg-status-error',
  };

  const pillColors = {
    active: 'bg-status-active/20 text-status-active',
    inactive: 'bg-status-inactive/20 text-status-inactive',
    installing: 'bg-status-warning/20 text-status-warning',
    error: 'bg-status-error/20 text-status-error',
  }

  return (
    <div
      className={`glass-card border-l-4 p-4 transition-all duration-300 hover:bg-accent/20 ${statusColors[worker.status]}`}
    >
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2">
          <div className="relative">
            <div
              className={`w-2.5 h-2.5 rounded-full ${statusIndicatorColors[worker.status]}`}
            />
            <div
                className={`absolute inset-0 w-2.5 h-2.5 rounded-full ${statusIndicatorColors[worker.status]} animate-ping`}
              />
          </div>
          <span className="font-semibold text-foreground">{worker.name}</span>
        </div>
        <span
          className={`text-xs px-2 py-1 rounded font-medium uppercase tracking-wide ${pillColors[worker.status]}`}
        >
          {worker.status}
        </span>
      </div>

      <div className="space-y-2 text-sm">
        <div className="flex items-center justify-between text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Activity className="w-3.5 h-3.5" />
            Address
          </span>
          <span className="font-mono text-foreground">
            {worker.tcpAddress}:{worker.port}
          </span>
        </div>

        <div className="flex items-center justify-between text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Gauge className="w-3.5 h-3.5" />
            Risk Multiplier
          </span>
          <span className="font-mono text-primary font-semibold">
            {worker.riskMultiplier}x
          </span>
        </div>

        <div className="flex items-center justify-between text-muted-foreground">
          <span className="flex items-center gap-1.5">
            {worker.mt5Connected ? (
              <Wifi className="w-3.5 h-3.5 text-status-active" />
            ) : (
              <WifiOff className="w-3.5 h-3.5 text-status-error" />
            )}
            MT5
          </span>
          <span
            className={`font-medium ${
              worker.mt5Connected ? 'text-status-active' : 'text-status-error'
            }`}
          >
            {worker.mt5Connected ? 'Connected' : 'Disconnected'}
          </span>
        </div>

        {worker.status === 'active' && (
          <div className="flex items-center justify-between text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <Clock className="w-3.5 h-3.5" />
              Latency
            </span>
            <span className="font-mono text-foreground">
              {formatLatency(worker.latency)}
            </span>
          </div>
        )}
      </div>

      <div className="mt-3 pt-3 border-t border-border/50">
        <div className="flex gap-2">
          {worker.status === 'active' ? (
            <button
              onClick={handleStop}
              className="flex-1 px-4 py-2 text-sm font-medium text-white bg-status-error rounded hover:bg-status-error/90 transition-colors"
            >
              Stop Worker
            </button>
          ) : (
            <button
              onClick={() => handleStart(false)}
              disabled={worker.status === 'installing' || worker.status === 'error'}
              className="flex-1 px-4 py-2 text-sm font-medium text-white bg-primary rounded hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-primary"
            >
              {worker.status === 'installing' ? 'Installing...' : 'Start Worker'}
            </button>
          )}
          {(worker.status === 'error' || worker.status === 'inactive') && (
            <button
              onClick={() => handleStart(true)}
              className="px-4 py-2 text-sm font-medium text-white bg-status-warning/80 rounded hover:bg-status-warning transition-colors"
              title="Force restart - kills any process using the worker's port and restarts"
            >
              <RotateCw className="w-4 h-4" />
            </button>
          )}
          <button
            onClick={() => setShowDeleteConfirm(true)}
            className="px-4 py-2 text-sm font-medium text-white bg-status-error/80 rounded hover:bg-status-error transition-colors"
            title="Delete Worker"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>

      {showDeleteConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="glass-card p-6 max-w-md w-full mx-4">
            <h3 className="text-lg font-semibold text-foreground mb-2">
              Delete Worker
            </h3>
            <p className="text-muted-foreground mb-6">
              Are you sure you want to delete worker <span className="font-semibold text-foreground">{worker.name}</span>? This action cannot be undone.
            </p>
            <div className="flex gap-3 justify-end">
              <button
                onClick={() => setShowDeleteConfirm(false)}
                className="px-4 py-2 text-sm font-medium text-foreground bg-accent/20 rounded hover:bg-accent/30 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleDelete}
                className="px-4 py-2 text-sm font-medium text-white bg-status-error rounded hover:bg-status-error/90 transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default WorkerCard;
