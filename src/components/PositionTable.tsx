import React, { useState } from 'react';
import { Position } from '@/types/trading';
import { formatPrice, formatLots, formatRelativeTime } from '@/lib/formatters';
import { ArrowUpRight, ArrowDownRight, Filter, Download } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { save } from '@tauri-apps/api/dialog';
import { writeTextFile } from '@tauri-apps/api/fs';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

interface PositionTableProps {
  positions: Position[];
  workers: { id: string; name: string }[];
}

const PositionTable: React.FC<PositionTableProps> = ({ positions, workers }) => {
  const [symbolFilter, setSymbolFilter] = useState('');
  const [workerFilter, setWorkerFilter] = useState('all');
  const [typeFilter, setTypeFilter] = useState<'all' | 'buy' | 'sell'>('all');

  const uniqueSymbols = [...new Set(positions.map(p => p.symbol))];

  const filteredPositions = positions.filter(position => {
    if (symbolFilter && !position.symbol.toLowerCase().includes(symbolFilter.toLowerCase())) {
      return false;
    }
    if (workerFilter !== 'all' && position.workerId !== workerFilter) {
      return false;
    }
    if (typeFilter !== 'all' && position.type !== typeFilter) {
      return false;
    }
    return true;
  });

  const handleExport = async () => {
    try {
      const csv = [
        ['Worker', 'Symbol', 'Type', 'Command', 'Entry', 'Lots', 'SL', 'TP', 'Age'].join(','),
        ...filteredPositions.map(p => [
          p.workerName,
          p.symbol,
          p.type,
          p.cmd,
          p.entryPrice,
          p.currentLots,
          p.sl,
          p.tp,
          formatRelativeTime(p.openTime),
        ].join(','))
      ].join('\n');

      const filePath = await save({
        defaultPath: `signals-${Date.now()}.csv`,
        filters: [{
          name: 'CSV',
          extensions: ['csv']
        }]
      });

      if (filePath) {
        await writeTextFile(filePath, csv);
      }
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to export CSV:', error);
      }
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/50">
        <h3 className="font-semibold text-foreground">Signals</h3>
        <div className="flex items-center gap-2">
          <Filter className="w-4 h-4 text-muted-foreground" />
          <span className="text-xs text-muted-foreground">
            {filteredPositions.length} of {positions.length}
          </span>
        </div>
      </div>

      <div className="flex items-center gap-3 px-4 py-3 bg-muted/20 border-b border-border/30">
        <Input
          placeholder="Filter symbol..."
          value={symbolFilter}
          onChange={(e) => setSymbolFilter(e.target.value)}
          className="h-8 w-32 bg-card border-border text-sm"
        />

        <Select value={workerFilter} onValueChange={setWorkerFilter}>
          <SelectTrigger className="h-8 w-36 bg-card border-border text-sm">
            <SelectValue placeholder="All Workers" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Workers</SelectItem>
            {workers.map(w => (
              <SelectItem key={w.id} value={w.id}>{w.name}</SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select value={typeFilter} onValueChange={(v) => setTypeFilter(v as 'all' | 'buy' | 'sell')}>
          <SelectTrigger className="h-8 w-28 bg-card border-border text-sm">
            <SelectValue placeholder="All Types" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="buy">BUY</SelectItem>
            <SelectItem value="sell">SELL</SelectItem>
          </SelectContent>
        </Select>

        <Button
          variant="outline"
          size="sm"
          onClick={handleExport}
          className="ml-auto h-8 text-xs"
        >
          <Download className="w-3 h-3 mr-1" />
          Export
        </Button>
      </div>

      <div className="flex-1 overflow-auto scrollbar-thin">
        <Table>
          <TableHeader className="sticky top-0 bg-card z-10">
            <TableRow className="border-border/50 hover:bg-transparent">
              <TableHead className="text-muted-foreground text-xs font-semibold">Worker</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold">Symbol</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold">Type</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold">Command</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold text-right">Entry</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold text-right">Lots</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold text-right">SL</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold text-right">TP</TableHead>
              <TableHead className="text-muted-foreground text-xs font-semibold">Age</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredPositions.map((position) => (
              <TableRow 
                key={position.id} 
                className="border-border/30 hover:bg-accent/30 transition-colors"
              >
                <TableCell className="font-medium text-sm">{position.workerName}</TableCell>
                <TableCell className="font-semibold">{position.symbol}</TableCell>
                <TableCell>
                  <div
                    className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-semibold ${
                      position.type === 'buy'
                        ? 'bg-buy/10 text-buy'
                        : 'bg-sell/10 text-sell'
                    }`}
                  >
                    {position.type === 'buy' ? (
                      <ArrowUpRight className="w-3 h-3" />
                    ) : (
                      <ArrowDownRight className="w-3 h-3" />
                    )}
                    {position.type !== null ? position.type.toUpperCase() : 'CLOSE'}
                  </div>
                </TableCell>
                <TableCell className={`font-mono text-sm text-right text-primary ${(position.cmd == 'open') ? 'text-success' : (position.cmd == 'modify') ?  'text-warning' : 'text-destructive'}`}>
                    {position.cmd.toUpperCase()}
                </TableCell>
                <TableCell className="font-mono text-sm text-right text-primary">
                  {formatPrice(position.entryPrice, position.symbol)}
                </TableCell>
                <TableCell className="font-mono text-sm text-right">
                  {formatLots(position.currentLots)}
                </TableCell>
                <TableCell className="font-mono text-sm text-right text-destructive">
                  {formatPrice(position.sl, position.symbol)}
                </TableCell>
                <TableCell className="font-mono text-sm text-right text-success">
                  {formatPrice(position.tp, position.symbol)}
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {formatRelativeTime(position.openTime)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
};

export default PositionTable;
