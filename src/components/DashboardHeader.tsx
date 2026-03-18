import React from 'react';
import { Menu, Rocket } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface DashboardHeaderProps {
  isConnected: boolean;
  onMenuClick?: () => void;
}

const DashboardHeader: React.FC<DashboardHeaderProps> = ({ 
  isConnected,
  onMenuClick,
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
            <a href="https://www.trading-rocket.nl/" target='_blank' rel="noopener noreferrer">
              <img 
                src="/logo.svg" 
                alt="Trading Rocket" 
                className="h-5 w-auto"
              />
            </a>
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

        <Button
          variant="default"
          size="sm"
          className="gap-2 bg-[#5865F2] hover:bg-[#4752C4] text-white"
          asChild
        >
          <a href="https://discord.com/invite/HV8ta8asQN" target="_blank" rel="noopener noreferrer">
            <span className="hidden md:inline">Join Discord</span>
            <Rocket className="w-4 h-4" />
          </a>
        </Button>
      </div>
    </header>
  );
};

export default DashboardHeader;
