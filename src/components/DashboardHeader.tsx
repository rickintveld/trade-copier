import React from 'react';
import { Activity, Menu } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface DashboardHeaderProps {
  isConnected: boolean;
  onMenuClick?: () => void;
}

const DashboardHeader: React.FC<DashboardHeaderProps> = ({ 
  isConnected,
  onMenuClick 
}) => {
  return (
    <header className="flex items-center justify-between px-6 py-4 bg-card/80 backdrop-blur-sm border-b border-border/50">
      <div className="flex items-center gap-4">
        <Button
          variant="ghost"
          size="icon"
          className="md:hidden"
          onClick={onMenuClick}
        >
          <Menu className="w-5 h-5" />
        </Button>
        
        <div className="flex items-center gap-3">
          <div className="relative">
            <img 
              src="/logo.svg" 
              alt="Trading Rocket" 
              className="h-8 w-auto"
            />
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-full bg-muted/50">
          <div className={`w-2 h-2 rounded-full ${
            isConnected ? 'bg-status-active animate-pulse' : 'bg-status-error'
          }`} />
          <span className="text-xs font-medium text-muted-foreground">
            {isConnected ? 'WebSocket Connected' : 'Disconnected'}
          </span>
        </div>

      </div>
    </header>
  );
};

export default DashboardHeader;
