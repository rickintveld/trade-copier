//+------------------------------------------------------------------+
//|                                            signal_receiver.mq5    |
//|                                         Trade Copier Slave EA     |
//|                                Receives trades from Rust Worker   |
//+------------------------------------------------------------------+
#property copyright "Trade Copier"
#property version   "1.00"
#property strict

#include <Trade\Trade.mqh>

#define INVALID_SOCKET -1              // Invalid socket handle

input string WorkerIP = "127.0.0.1";   // Rust Worker IP
input int WorkerPort = 5050;           // Rust Worker Port
input int MagicNumber = 999888;        // Magic Number for trades
input int Slippage = 10;               // Slippage in points

int socketHandle = INVALID_SOCKET;
CTrade trade;

// Position tracking: maps trade_id to position ticket
ulong g_trade_ids[];
ulong g_position_tickets[];
int g_tracking_count = 0;

// Helper functions
int FindTradeIdIndex(ulong trade_id);
void AddPositionMapping(ulong trade_id, ulong ticket);
void RemovePositionMapping(ulong trade_id);

//+------------------------------------------------------------------+
//| Expert initialization function                                   |
//+------------------------------------------------------------------+
int OnInit()
{
   // Initialize tracking arrays
   ArrayResize(g_trade_ids, 0);
   ArrayResize(g_position_tickets, 0);
   g_tracking_count = 0;
   
   Print("[RECEIVER] Trade Copier Slave EA started");
   Print("[RECEIVER] Connecting to worker at ", WorkerIP, ":", WorkerPort);
   
   // Initialize TCP socket
   socketHandle = SocketCreate();
   if(socketHandle == INVALID_SOCKET)
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Failed to create socket, error code: ", error);
      return INIT_FAILED;
   }
   
   Print("[RECEIVER] Socket created successfully, handle: ", socketHandle);
   
   // Connect to Rust worker's TCP server
   if(!SocketConnect(socketHandle, WorkerIP, WorkerPort, 1000))
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Failed to connect to worker at ", WorkerIP, ":", WorkerPort, ", error code: ", error);
      Print("[RECEIVER] Common error codes: 5002=DLL not allowed, 4014=Internal error, 5200=Socket error");
      SocketClose(socketHandle);
      return INIT_FAILED;
   }
   
   // Set trade parameters
   trade.SetExpertMagicNumber(MagicNumber);
   trade.SetDeviationInPoints(Slippage);
   trade.SetTypeFilling(ORDER_FILLING_FOK);
   
   Print("[RECEIVER] Connected to worker successfully (TCP)");
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                 |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   if(socketHandle != INVALID_SOCKET)
      SocketClose(socketHandle);
   
   Print("[RECEIVER] Slave EA stopped");
}

//+------------------------------------------------------------------+
//| Expert tick function                                             |
//+------------------------------------------------------------------+
void OnTick()
{
   // Check for incoming TCP messages
   CheckIncomingTrades();
}

//+------------------------------------------------------------------+
//| Check for incoming trades                                        |
//+------------------------------------------------------------------+
void CheckIncomingTrades()
{
   // Check if data is available
   uint len = SocketIsReadable(socketHandle);
   if(len == 0)
      return;
   
   // Receive data
   uchar buffer[];
   ArrayResize(buffer, len);
   
   int received = SocketRead(socketHandle, buffer, len, 0);
   
   if(received > 0)
   {
      // Convert to string
      string data = CharArrayToString(buffer, 0, received);
      Print("[RECEIVER] Received packet (", received, " bytes): ", data);
      
      // Parse and execute trade (may contain multiple newline-delimited messages)
      string messages[];
      int count = StringSplit(data, '\n', messages);
      
      for(int i = 0; i < count; i++)
      {
         if(StringLen(messages[i]) > 0)
         {
            if(ParseAndExecuteTrade(messages[i]))
               Print("[RECEIVER] Trade executed successfully");
            else
               Print("[RECEIVER] Failed to execute trade");
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Parse JSON and execute trade                                     |
//+------------------------------------------------------------------+
bool ParseAndExecuteTrade(string json_data)
{
   // Simple JSON parsing (in production, use a proper JSON library)
   ulong trade_id = 0;
   string symbol = "";
   string trade_type = "";
   double lots = 0.0;
   double price = 0.0;
   double sl = 0.0;
   double tp = 0.0;
   string cmd = "open";  // Default to open
   
   // Extract fields from JSON
   if(!ExtractJSONField(json_data, "id", trade_id)) return false;
   if(!ExtractJSONField(json_data, "symbol", symbol)) return false;
   if(!ExtractJSONField(json_data, "type", trade_type)) return false;
   if(!ExtractJSONField(json_data, "lots", lots)) return false;
   
   ExtractJSONField(json_data, "price", price);
   ExtractJSONField(json_data, "sl", sl);
   ExtractJSONField(json_data, "tp", tp);
   ExtractJSONField(json_data, "cmd", cmd);
   
   Print("[RECEIVER] Parsed trade: ID=", trade_id, " Symbol=", symbol, " Cmd=", cmd);
   
   // Handle different commands
   bool success = false;
   
   if(cmd == "open")
   {
      // Execute new position
      if(trade_type == "buy")
      {
         success = trade.Buy(lots, symbol, 0, sl, tp, "CopiedTrade");
      }
      else if(trade_type == "sell")
      {
         success = trade.Sell(lots, symbol, 0, sl, tp, "CopiedTrade");
      }
      
      if(success)
      {
         ulong ticket = trade.ResultOrder();
         // Get actual position ticket (not order ticket)
         if(PositionSelectByTicket(ticket))
            ticket = PositionGetInteger(POSITION_TICKET);
         else if(PositionSelect(symbol))
            ticket = PositionGetInteger(POSITION_TICKET);
            
         AddPositionMapping(trade_id, ticket);
         Print("[RECEIVER] Position opened: ticket=", ticket, " trade_id=", trade_id);
         return true;
      }
      else
      {
         Print("[RECEIVER] Failed to open position: ", trade.ResultRetcodeDescription());
         return false;
      }
   }
   else if(cmd == "close")
   {
      // Close existing position
      int idx = FindTradeIdIndex(trade_id);
      if(idx < 0)
      {
         Print("[RECEIVER] WARNING: Cannot close - trade_id ", trade_id, " not found in tracking");
         return false;
      }
      
      ulong ticket = g_position_tickets[idx];
      success = trade.PositionClose(ticket);
      
      if(success)
      {
         Print("[RECEIVER] Position closed: ticket=", ticket, " trade_id=", trade_id);
         RemovePositionMapping(trade_id);
         return true;
      }
      else
      {
         Print("[RECEIVER] Failed to close position: ", trade.ResultRetcodeDescription());
         return false;
      }
   }
   else if(cmd == "modify")
   {
      // Modify existing position SL/TP
      int idx = FindTradeIdIndex(trade_id);
      if(idx < 0)
      {
         Print("[RECEIVER] WARNING: Cannot modify - trade_id ", trade_id, " not found in tracking");
         return false;
      }
      
      ulong ticket = g_position_tickets[idx];
      
      // Get current position symbol for PositionModify
      if(!PositionSelectByTicket(ticket))
      {
         Print("[RECEIVER] WARNING: Position ticket ", ticket, " no longer exists");
         RemovePositionMapping(trade_id);
         return false;
      }
      
      success = trade.PositionModify(ticket, sl, tp);
      
      if(success)
      {
         Print("[RECEIVER] Position modified: ticket=", ticket, " SL=", sl, " TP=", tp);
         return true;
      }
      else
      {
         Print("[RECEIVER] Failed to modify position: ", trade.ResultRetcodeDescription());
         return false;
      }
   }
   else
   {
      Print("[RECEIVER] ERROR: Unknown command: ", cmd);
      return false;
   }
}

//+------------------------------------------------------------------+
//| Extract field from JSON (simple parser) - ulong overload        |
//+------------------------------------------------------------------+
bool ExtractJSONField(string json, string field_name, ulong &value)
{
   string search = "\"" + field_name + "\":";
   int pos = StringFind(json, search);
   
   if(pos < 0) return false;
   
   pos += StringLen(search);
   
   // Skip whitespace and quotes
   while(pos < StringLen(json) && (StringGetCharacter(json, pos) == ' ' || StringGetCharacter(json, pos) == '"'))
      pos++;
   
   // Extract value
   string value_str = "";
   while(pos < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, pos);
      if(ch == ',' || ch == '}' || ch == '"')
         break;
      value_str += ShortToString(ch);
      pos++;
   }
   
   value_str = StringTrimLeft(value_str);
   value_str = StringTrimRight(value_str);
   
   value = (ulong)StringToInteger(value_str);
   return StringLen(value_str) > 0;
}

//+------------------------------------------------------------------+
//| Extract field from JSON (simple parser) - double overload       |
//+------------------------------------------------------------------+
bool ExtractJSONField(string json, string field_name, double &value)
{
   string search = "\"" + field_name + "\":";
   int pos = StringFind(json, search);
   
   if(pos < 0) return false;
   
   pos += StringLen(search);
   
   // Skip whitespace and quotes
   while(pos < StringLen(json) && (StringGetCharacter(json, pos) == ' ' || StringGetCharacter(json, pos) == '"'))
      pos++;
   
   // Extract value
   string value_str = "";
   while(pos < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, pos);
      if(ch == ',' || ch == '}' || ch == '"')
         break;
      value_str += ShortToString(ch);
      pos++;
   }
   
   value_str = StringTrimLeft(value_str);
   value_str = StringTrimRight(value_str);
   
   value = StringToDouble(value_str);
   return StringLen(value_str) > 0;
}

//+------------------------------------------------------------------+
//| Extract field from JSON (simple parser) - string overload       |
//+------------------------------------------------------------------+
bool ExtractJSONField(string json, string field_name, string &value)
{
   string search = "\"" + field_name + "\":";
   int pos = StringFind(json, search);
   
   if(pos < 0) return false;
   
   pos += StringLen(search);
   
   // Skip whitespace and quotes
   while(pos < StringLen(json) && (StringGetCharacter(json, pos) == ' ' || StringGetCharacter(json, pos) == '"'))
      pos++;
   
   // Extract value
   string value_str = "";
   while(pos < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, pos);
      if(ch == ',' || ch == '}' || ch == '"')
         break;
      value_str += ShortToString(ch);
      pos++;
   }
   
   value = StringTrimLeft(value_str);
   value = StringTrimRight(value_str);
   
   return StringLen(value) > 0;
}

//+------------------------------------------------------------------+
//| Position tracking helper functions                               |
//+------------------------------------------------------------------+
int FindTradeIdIndex(ulong trade_id)
{
   for(int i = 0; i < g_tracking_count; i++)
   {
      if(g_trade_ids[i] == trade_id)
         return i;
   }
   return -1;
}

void AddPositionMapping(ulong trade_id, ulong ticket)
{
   // Check if already exists (shouldn't happen, but be safe)
   int idx = FindTradeIdIndex(trade_id);
   if(idx >= 0)
   {
      // Update existing mapping
      g_position_tickets[idx] = ticket;
      return;
   }
   
   g_tracking_count++;
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_tickets, g_tracking_count);
   
   g_trade_ids[g_tracking_count - 1] = trade_id;
   g_position_tickets[g_tracking_count - 1] = ticket;
   
   Print("[RECEIVER] Mapped trade_id=", trade_id, " to ticket=", ticket);
}

void RemovePositionMapping(ulong trade_id)
{
   int idx = FindTradeIdIndex(trade_id);
   if(idx < 0) return;
   
   // Shift arrays to remove element
   for(int i = idx; i < g_tracking_count - 1; i++)
   {
      g_trade_ids[i] = g_trade_ids[i + 1];
      g_position_tickets[i] = g_position_tickets[i + 1];
   }
   
   g_tracking_count--;
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_tickets, g_tracking_count);
   
   Print("[RECEIVER] Removed mapping for trade_id=", trade_id);
}
