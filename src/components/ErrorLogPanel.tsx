import React, { useState } from 'react';
import { ErrorLog } from '@/types/trading';
import { formatDateTime } from '@/lib/formatters';
import { AlertTriangle, AlertCircle, XCircle, Search, Filter } from 'lucide-react';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface ErrorLogPanelProps {
  errors: ErrorLog[];
}

const SeverityIcon: React.FC<{ severity: ErrorLog['severity'] }> = ({ severity }) => {
  switch (severity) {
    case 'warning':
      return <AlertTriangle className="w-4 h-4 text-warning" />;
    case 'error':
      return <AlertCircle className="w-4 h-4 text-destructive" />;
    case 'critical':
      return <XCircle className="w-4 h-4 text-destructive animate-pulse" />;
  }
};

const ErrorLogPanel: React.FC<ErrorLogPanelProps> = ({ errors }) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [severityFilter, setSeverityFilter] = useState<'all' | ErrorLog['severity']>('all');
  const [typeFilter, setTypeFilter] = useState<'all' | ErrorLog['type']>('all');

  const filteredErrors = errors.filter(error => {
    if (searchQuery && !error.message.toLowerCase().includes(searchQuery.toLowerCase())) {
      return false;
    }
    if (severityFilter !== 'all' && error.severity !== severityFilter) {
      return false;
    }
    if (typeFilter !== 'all' && error.type !== typeFilter) {
      return false;
    }
    return true;
  });

  const severityCounts = {
    warning: errors.filter(e => e.severity === 'warning').length,
    error: errors.filter(e => e.severity === 'error').length,
    critical: errors.filter(e => e.severity === 'critical').length,
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/50">
        <h3 className="font-semibold text-foreground">Error Log</h3>
        <div className="flex items-center gap-3">
          {severityCounts.critical > 0 && (
            <span className="flex items-center gap-1 text-xs">
              <XCircle className="w-3 h-3 text-destructive" />
              <span className="text-destructive">{severityCounts.critical}</span>
            </span>
          )}
          {severityCounts.error > 0 && (
            <span className="flex items-center gap-1 text-xs">
              <AlertCircle className="w-3 h-3 text-destructive/70" />
              <span className="text-destructive/70">{severityCounts.error}</span>
            </span>
          )}
          {severityCounts.warning > 0 && (
            <span className="flex items-center gap-1 text-xs">
              <AlertTriangle className="w-3 h-3 text-warning" />
              <span className="text-warning">{severityCounts.warning}</span>
            </span>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3 px-4 py-3 bg-muted/20 border-b border-border/30">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            placeholder="Search errors..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-8 pl-9 bg-card border-border text-sm"
          />
        </div>

        <Select 
          value={severityFilter} 
          onValueChange={(v) => setSeverityFilter(v as 'all' | ErrorLog['severity'])}
        >
          <SelectTrigger className="h-8 w-32 bg-card border-border text-sm">
            <SelectValue placeholder="Severity" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Severity</SelectItem>
            <SelectItem value="warning">Warning</SelectItem>
            <SelectItem value="error">Error</SelectItem>
            <SelectItem value="critical">Critical</SelectItem>
          </SelectContent>
        </Select>

        <Select 
          value={typeFilter} 
          onValueChange={(v) => setTypeFilter(v as 'all' | ErrorLog['type'])}
        >
          <SelectTrigger className="h-8 w-32 bg-card border-border text-sm">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="connection">Connection</SelectItem>
            <SelectItem value="parse">Parse</SelectItem>
            <SelectItem value="send">Send</SelectItem>
            <SelectItem value="channel">Channel</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-thin">
        {filteredErrors.map((error) => (
          <div
            key={error.id}
            className={`data-row flex items-start gap-3 animate-slide-up ${
              error.severity === 'critical' ? 'bg-destructive/5' : ''
            }`}
          >
            <SeverityIcon severity={error.severity} />
            
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span className="font-mono text-xs text-muted-foreground">
                  {formatDateTime(error.timestamp)}
                </span>
                <span
                  className={`px-1.5 py-0.5 rounded text-xs font-medium uppercase tracking-wide ${
                    error.severity === 'critical'
                      ? 'bg-destructive/20 text-destructive'
                      : error.severity === 'error'
                      ? 'bg-destructive/10 text-destructive/80'
                      : 'bg-warning/20 text-warning'
                  }`}
                >
                  {error.severity}
                </span>
                <span className="px-1.5 py-0.5 rounded text-xs bg-muted text-muted-foreground uppercase">
                  {error.type}
                </span>
                {error.workerName && (
                  <span className="text-xs text-primary">{error.workerName}</span>
                )}
              </div>
              <p className="text-sm text-foreground">{error.message}</p>
              {error.details && (
                <p className="text-xs text-muted-foreground mt-1">{error.details}</p>
              )}
            </div>
          </div>
        ))}

        {filteredErrors.length === 0 && (
          <div className="flex items-center justify-center h-32 text-muted-foreground text-sm">
            No errors matching filters
          </div>
        )}
      </div>
    </div>
  );
};

export default ErrorLogPanel;
