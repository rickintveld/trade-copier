import React from 'react';
import { Trade } from '@/types/trading';
import { formatTimestamp, formatLots, formatPrice } from '@/lib/formatters';
import { ChevronDown, ChevronRight, ArrowUpRight, ArrowDownRight } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';

interface TradeActivityFeedProps {
  trades: Trade[];
}

const CommandBadge: React.FC<{ command: Trade['command'] }> = ({ command }) => {
  const styles = {
    OPEN: 'bg-open/20 text-open border-open/30',
    CLOSE: 'bg-close/20 text-close border-close/30',
    MODIFY: 'bg-modify/20 text-modify border-modify/30',
  };

  return (
    <Badge variant="outline" className={`${styles[command]} font-mono text-xs`}>
      {command}
    </Badge>
  );
};

const TypeBadge: React.FC<{ type: Trade['type'] }> = ({ type }) => {
  return (
    <div
      className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs font-semibold ${
        type === 'BUY'
          ? 'bg-buy/10 text-buy'
          : 'bg-sell/10 text-sell'
      }`}
    >
      {type === 'BUY' ? (
        <ArrowUpRight className="w-3 h-3" />
      ) : (
        <ArrowDownRight className="w-3 h-3" />
      )}
      {type}
    </div>
  );
};

const TradeRow: React.FC<{ trade: Trade }> = ({ trade }) => {
  const [isOpen, setIsOpen] = React.useState(false);

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger asChild>
        <div className="data-row flex items-center gap-4 cursor-pointer group">
          <div className="flex items-center gap-2 text-muted-foreground">
            {isOpen ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
            <span className="font-mono text-xs text-muted-foreground">
              {formatTimestamp(trade.timestamp)}
            </span>
          </div>

          <span className="font-mono text-xs text-muted-foreground/60">
            {trade.id}
          </span>

          <div className="flex items-center gap-2 min-w-[100px]">
            <span className="font-semibold text-foreground">{trade.symbol}</span>
            <TypeBadge type={trade.type} />
          </div>

          <CommandBadge command={trade.command} />

          <div className="flex-1 flex items-center gap-4 text-sm font-mono">
            <span className="text-muted-foreground">
              Lots: <span className="text-foreground">{formatLots(trade.originalLots)}</span>
            </span>
            <span className="text-muted-foreground">
              @ <span className="text-primary">{formatPrice(trade.price, trade.symbol)}</span>
            </span>
          </div>

          <div className="flex items-center gap-1">
            {trade.workerResults.slice(0, 3).map((result) => (
              <div
                key={result.workerId}
                className={`w-6 h-6 rounded text-xs font-semibold flex items-center justify-center ${
                  result.status === 'success'
                    ? 'bg-status-active/20 text-status-active'
                    : result.status === 'pending'
                    ? 'bg-warning/20 text-warning'
                    : 'bg-status-error/20 text-status-error'
                }`}
                title={`${result.workerName}: ${result.adjustedLots} lots`}
              >
                {result.workerName.slice(-1)}
              </div>
            ))}
            {trade.workerResults.length > 3 && (
              <span className="text-xs text-muted-foreground">
                +{trade.workerResults.length - 3}
              </span>
            )}
          </div>
        </div>
      </CollapsibleTrigger>

      <CollapsibleContent>
        <div className="bg-muted/20 px-6 py-4 border-b border-border/30">
          <div className="grid grid-cols-4 gap-4 mb-4 text-sm">
            <div>
              <span className="text-muted-foreground block text-xs mb-1">Stop Loss</span>
              <span className="font-mono text-destructive">
                {formatPrice(trade.sl, trade.symbol)}
              </span>
            </div>
            <div>
              <span className="text-muted-foreground block text-xs mb-1">Take Profit</span>
              <span className="font-mono text-success">
                {formatPrice(trade.tp, trade.symbol)}
              </span>
            </div>
          </div>

          <div>
            <span className="text-xs text-muted-foreground mb-2 block">Worker Distribution</span>
            <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
              {trade.workerResults.map((result) => (
                <div
                  key={result.workerId}
                  className="flex items-center justify-between bg-card/50 rounded px-3 py-2"
                >
                  <span className="font-medium text-sm">{result.workerName}</span>
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs text-muted-foreground">
                      {formatLots(trade.originalLots)} →
                    </span>
                    <span className="font-mono text-sm text-primary font-semibold">
                      {formatLots(result.adjustedLots)}
                    </span>
                    <div
                      className={`w-2 h-2 rounded-full ${
                        result.status === 'success'
                          ? 'bg-status-active'
                          : result.status === 'pending'
                          ? 'bg-warning'
                          : 'bg-status-error'
                      }`}
                    />
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
};

const TradeActivityFeed: React.FC<TradeActivityFeedProps> = ({ trades }) => {
  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/50">
        <h3 className="font-semibold text-foreground">Live Trade Activity</h3>
        <span className="text-xs text-muted-foreground">
          {trades.length} trades
        </span>
      </div>
      <div className="flex-1 overflow-y-auto scrollbar-thin">
        {trades.map((trade) => (
          <TradeRow key={trade.id} trade={trade} />
        ))}
      </div>
    </div>
  );
};

export default TradeActivityFeed;
