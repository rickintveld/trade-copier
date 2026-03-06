import React, { useState, useEffect } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { Worker } from '@/types/trading';
import WorkerCard from './WorkerCard';
import WorkerFormDialog from './WorkerFormDialog';
import { Users, ChevronLeft, ChevronRight, Plus, BookOpen, Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { DependencyCheckModal } from './DependencyCheckModal';
import { SettingsModal } from './SettingsModal';

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
  const [appVersion, setAppVersion] = useState<string>('');
  const [isSetupGuideOpen, setIsSetupGuideOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const activeCount = workers.filter(w => w.status === 'active').length;
  const errorCount = workers.filter(w => w.status === 'error').length;

  useEffect(() => {
    getVersion().then(version => setAppVersion(version));
  }, []);

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

          <div className="px-3 py-2 space-y-2">
            <Button className="w-full" variant="outline" size="sm" onClick={() => setIsDialogOpen(true)}>
              <Plus className="w-4 h-4 mr-2" />
              Add Worker
            </Button>
            <WorkerFormDialog
              mode="create"
              open={isDialogOpen}
              onOpenChange={setIsDialogOpen}
              onSuccess={onWorkerCreated}
            />
          </div>
          
          <DependencyCheckModal 
            isOpen={isSetupGuideOpen} 
            onOpenChange={setIsSetupGuideOpen}
          />
          <SettingsModal
            isOpen={isSettingsOpen}
            onOpenChange={setIsSettingsOpen}
          />

          <div className="flex-1 overflow-y-auto scrollbar-thin p-3 space-y-3">
            {workers.map((worker) => (
              <WorkerCard key={worker.id} worker={worker} onWorkerUpdated={onWorkerCreated} />
            ))}
          </div>

          <div className="px-3 pb-2 space-y-1">
            <Button 
              className="w-full" 
              variant="ghost" 
              size="sm"
              onClick={() => setIsSettingsOpen(true)}
            >
              <Settings className="w-4 h-4 mr-2" />
              Settings
            </Button>
            <Button 
              className="w-full" 
              variant="ghost" 
              size="sm"
              onClick={() => setIsSetupGuideOpen(true)}
            >
              <BookOpen className="w-4 h-4 mr-2" />
              Setup Guide
            </Button>
          </div>

          {appVersion && (
            <div className="mt-auto p-3 border-t border-border/50">
              <div className="flex flex-col gap-2">
                <p className="text-xs text-muted-foreground text-center">
                  <a href="https://trading-rocket.nl/" target='_blank'>Trading Rocket v{appVersion}</a>
                </p>
              </div>
            </div>
          )}
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
                {worker.port}
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
