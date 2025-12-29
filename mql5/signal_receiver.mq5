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
bool g_connection_lost = false;
datetime g_last_recv_time = 0;

// Position tracking: maps trade_id to position ticket
ulong g_trade_ids[];
ulong g_position_tickets[];
int g_tracking_count = 0;

// Helper functions
int FindTradeIdIndex(ulong trade_id);
void AddPositionMapping(ulong trade_id, ulong ticket);
void RemovePositionMapping(ulong trade_id);
void SendAcknowledgment(bool success, string message);

// Connection management
bool ConnectToWorker();
void DisconnectFromWorker();
bool EnsureConnection();

//+------------------------------------------------------------------+
//| Expert initialization function                                   |
//+------------------------------------------------------------------+
int OnInit()
{
   // Initialize tracking arrays
   ArrayResize(g_trade_ids, 0);
   ArrayResize(g_position_tickets, 0);
   g_tracking_count = 0;
   g_connection_lost = false;
   g_last_recv_time = 0;
   
   Print("[RECEIVER] Trade Copier Slave EA started");
   Print("[RECEIVER] Connecting to worker at ", WorkerIP, ":", WorkerPort);
   
   // Set trade parameters
   trade.SetExpertMagicNumber(MagicNumber);
   trade.SetDeviationInPoints(Slippage);
   trade.SetTypeFilling(ORDER_FILLING_FOK);
   
   if(!ConnectToWorker())
      return INIT_FAILED;
   
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                 |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   DisconnectFromWorker();
   Print("[RECEIVER] Slave EA stopped");
}

//+------------------------------------------------------------------+
//| Expert tick function                                             |
//+------------------------------------------------------------------+
void OnTick()
{
   // Reconnect if connection lost
   if(g_connection_lost)
   {
      datetime current_time = TimeLocal();
      if(current_time - g_last_recv_time > 5) // Try reconnect every 5 seconds
      {
         Print("[RECEIVER] Attempting to reconnect...");
         DisconnectFromWorker();
         g_connection_lost = !ConnectToWorker();
         g_last_recv_time = current_time;
      }
   }
   
   // Check for incoming TCP messages
   CheckIncomingTrades();
}

//+------------------------------------------------------------------+
//| Check for incoming trades                                        |
//+------------------------------------------------------------------+
void CheckIncomingTrades()
{
   if(!EnsureConnection())
      return;
   
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
      g_last_recv_time = TimeLocal();
      
      // Convert to string
      string data = CharArrayToString(buffer, 0, received, CP_UTF8);
      
      // Parse and execute trade (may contain multiple newline-delimited messages)
      string messages[];
      int count = StringSplit(data, '\n', messages);
      
      for(int i = 0; i < count; i++)
      {
         if(StringLen(messages[i]) > 0)
            ParseAndExecuteTrade(messages[i]);
      }
   }
   else if(received < 0)
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Read failed, error: ", error);
      if(error == 5273) // ERR_NETSOCKET_IO_ERROR
      {
         Print("[RECEIVER] Connection lost (ERR_NETSOCKET_IO_ERROR). Will attempt reconnect.");
         g_connection_lost = true;
      }
   }
}

//+------------------------------------------------------------------+
//| Parse JSON and execute trade                                     |
//+------------------------------------------------------------------+
bool ParseAndExecuteTrade(string json_data)
{
   
   ulong trade_id = 0;
   string symbol = "";
   string trade_type = "";
   double lots = 0.0;
   double price = 0.0;
   double sl = 0.0;
   double tp = 0.0;
   string cmd = "open";  // Default to open
   
   // Extract fields from JSON
   if(!ExtractJSONULong(json_data, "id", trade_id)) 
   {
      SendAcknowledgment(false, "Failed to parse trade_id");
      return false;
   }
   if(!ExtractJSONString(json_data, "symbol", symbol)) 
   {
      SendAcknowledgment(false, "Failed to parse symbol");
      return false;
   }
   if(!ExtractJSONDouble(json_data, "lots", lots)) 
   {
      SendAcknowledgment(false, "Failed to parse lots");
      return false;
   }
   
   // Optional fields
   ExtractJSONString(json_data, "type", trade_type);  // Optional - not needed for close/modify
   ExtractJSONDouble(json_data, "sl", sl);
   ExtractJSONDouble(json_data, "tp", tp);
   ExtractJSONString(json_data, "cmd", cmd);
   
   // Handle different commands
   bool success = false;
   
   if(cmd == "open")
   {
      // Validate trade_type for open command
      if(trade_type == "")
      {
         SendAcknowledgment(false, "trade_type required for open command");
         return false;
      }
      
      // Execute new position
      if(trade_type == "buy")
      {
         success = trade.Buy(lots, symbol, 0, sl, tp, "CopiedTrade");
      }
      else if(trade_type == "sell")
      {
         success = trade.Sell(lots, symbol, 0, sl, tp, "CopiedTrade");
      }
      else
      {
         SendAcknowledgment(false, "Invalid trade_type: " + trade_type);
         return false;
      }
      
      if(success)
      {
         // Get the position ticket from the result
         ulong ticket = trade.ResultDeal();
         if(ticket > 0 && HistoryDealSelect(ticket))
         {
            ticket = HistoryDealGetInteger(ticket, DEAL_POSITION_ID);
         }
         else
         {
            // Fallback: try ResultOrder
            ticket = trade.ResultOrder();
         }
         
         if(ticket > 0)
         {
            AddPositionMapping(trade_id, ticket);
            SendAcknowledgment(true, "Trade opened successfully");
         }
         else
         {
            SendAcknowledgment(false, "Failed to get position ticket");
         }
      }
      else
      {
         SendAcknowledgment(false, "Failed to open trade");
      }
      return success;
   }
   else if(cmd == "close")
   {
      int idx = FindTradeIdIndex(trade_id);
      if(idx < 0) 
      {
         SendAcknowledgment(false, "Trade ID not found");
         return false;
      }
      
      success = trade.PositionClose(g_position_tickets[idx]);
      if(success)
      {
         RemovePositionMapping(trade_id);
         SendAcknowledgment(true, "Trade closed successfully");
      }
      else
      {
         SendAcknowledgment(false, "Failed to close trade");
      }
      return success;
   }
   else if(cmd == "partial_close")
   {
      int idx = FindTradeIdIndex(trade_id);
      if(idx < 0) 
      {
         SendAcknowledgment(false, "Trade ID not found");
         return false;
      }
      
      ulong ticket = g_position_tickets[idx];
      if(!PositionSelectByTicket(ticket))
      {
         RemovePositionMapping(trade_id);
         SendAcknowledgment(false, "Position not found");
         return false;
      }
      
      // Partial close: close specified volume, keep position mapping
      success = trade.PositionClosePartial(ticket, lots);
      if(success)
      {
         SendAcknowledgment(true, "Partial close successful");
      }
      else
      {
         SendAcknowledgment(false, "Failed to partially close trade");
      }
      return success;
   }
   else if(cmd == "modify")
   {
      int idx = FindTradeIdIndex(trade_id);
      if(idx < 0) 
      {
         SendAcknowledgment(false, "Trade ID not found");
         return false;
      }
      
      ulong ticket = g_position_tickets[idx];
      if(!PositionSelectByTicket(ticket))
      {
         RemovePositionMapping(trade_id);
         SendAcknowledgment(false, "Position not found");
         return false;
      }
      success = trade.PositionModify(ticket, sl, tp);
      if(success)
      {
         SendAcknowledgment(true, "Trade modified successfully");
      }
      else
      {
         SendAcknowledgment(false, "Failed to modify trade");
      }
      return success;
   }
   SendAcknowledgment(false, "Unknown command");
   return false;
}

//+------------------------------------------------------------------+
//| Extract field from JSON (simple parser) - ulong version         |
//+------------------------------------------------------------------+
bool ExtractJSONULong(string json, string field_name, ulong &value)
{
   string search = "\"" + field_name + "\":";
   int pos = StringFind(json, search);
   
   if(pos < 0) return false;
   
   pos += StringLen(search);
   
   // Skip whitespace
   while(pos < StringLen(json) && StringGetCharacter(json, pos) == ' ')
      pos++;
   
   // Find end of value
   int start_pos = pos;
   while(pos < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, pos);
      if(ch == ',' || ch == '}' || ch == ' ')
         break;
      pos++;
   }
   
   // Parse ulong manually to handle large numbers
   value = 0;
   while(start_pos < pos)
   {
      ushort digit = StringGetCharacter(json, start_pos);
      if(digit >= '0' && digit <= '9')
         value = value * 10 + (digit - '0');
      start_pos++;
   }
   
   return value > 0;
}

//+------------------------------------------------------------------+
//| Extract field from JSON (simple parser) - double version        |
//+------------------------------------------------------------------+
bool ExtractJSONDouble(string json, string field_name, double &value)
{
   string search = "\"" + field_name + "\":";
   int pos = StringFind(json, search);
   
   if(pos < 0) return false;
   
   pos += StringLen(search);
   
   // Skip whitespace
   while(pos < StringLen(json) && StringGetCharacter(json, pos) == ' ')
      pos++;
   
   // Find end of value
   int start_pos = pos;
   while(pos < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, pos);
      if(ch == ',' || ch == '}' || ch == ' ')
         break;
      pos++;
   }
   
   value = StringToDouble(StringSubstr(json, start_pos, pos - start_pos));
   return pos > start_pos;
}

//+------------------------------------------------------------------+
//| Extract field from JSON (simple parser) - string version        |
//+------------------------------------------------------------------+
bool ExtractJSONString(string json, string field_name, string &value)
{
   string search = "\"" + field_name + "\":";
   int pos = StringFind(json, search);
   
   if(pos < 0) return false;
   
   pos += StringLen(search);
   
   // Skip whitespace
   while(pos < StringLen(json) && StringGetCharacter(json, pos) == ' ')
      pos++;
   
   if(StringGetCharacter(json, pos) == '"')
      pos++; // Skip opening quote
   
   int start_pos = pos;
   while(pos < StringLen(json))
   {
      ushort ch = StringGetCharacter(json, pos);
      if(ch == '"' || ch == ',' || ch == '}')
         break;
      pos++;
   }
   
   value = StringSubstr(json, start_pos, pos - start_pos);
   return pos > start_pos;
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
   int idx = FindTradeIdIndex(trade_id);
   if(idx >= 0)
   {
      g_position_tickets[idx] = ticket;
      return;
   }
   
   g_tracking_count++;
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_tickets, g_tracking_count);
   g_trade_ids[g_tracking_count - 1] = trade_id;
   g_position_tickets[g_tracking_count - 1] = ticket;
}

void RemovePositionMapping(ulong trade_id)
{
   int idx = FindTradeIdIndex(trade_id);
   if(idx < 0) return;
   
   for(int i = idx; i < g_tracking_count - 1; i++)
   {
      g_trade_ids[i] = g_trade_ids[i + 1];
      g_position_tickets[i] = g_position_tickets[i + 1];
   }
   g_tracking_count--;
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_tickets, g_tracking_count);
}

//+------------------------------------------------------------------+
//| Connection management functions                                  |
//+------------------------------------------------------------------+
bool ConnectToWorker()
{
   // Initialize TCP socket
   socketHandle = SocketCreate();
   if(socketHandle == INVALID_SOCKET)
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Failed to create socket, error code: ", error);
      return false;
   }
   
   Print("[RECEIVER] Socket created successfully, handle: ", socketHandle);
   
   // Connect to Rust worker's TCP server
   if(!SocketConnect(socketHandle, WorkerIP, WorkerPort, 1000))
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Failed to connect to worker at ", WorkerIP, ":", WorkerPort, ", error code: ", error);
      Print("[RECEIVER] Common error codes: 5002=DLL not allowed, 4014=Internal error, 5200=Socket error");
      SocketClose(socketHandle);
      socketHandle = INVALID_SOCKET;
      return false;
   }
   
   Print("[RECEIVER] Connected to worker successfully (TCP)");
   g_connection_lost = false;
   g_last_recv_time = TimeLocal();
   return true;
}

void DisconnectFromWorker()
{
   if(socketHandle != INVALID_SOCKET)
   {
      SocketClose(socketHandle);
      socketHandle = INVALID_SOCKET;
      Print("[RECEIVER] Disconnected from worker");
   }
}

bool EnsureConnection()
{
   if(g_connection_lost)
      return false;
      
   if(socketHandle == INVALID_SOCKET)
   {
      g_connection_lost = true;
      return false;
   }
   
   return true;
}

//+------------------------------------------------------------------+
//| Send acknowledgment back to Rust worker                          |
//+------------------------------------------------------------------+
void SendAcknowledgment(bool success, string message)
{
   if(!EnsureConnection())
      return;
   
   // Create acknowledgment message (newline-terminated)
   string ack = (success ? "OK: " : "ERROR: ") + message + "\n";
   
   // Convert to bytes
   uchar data[];
   StringToCharArray(ack, data, 0, StringLen(ack), CP_UTF8);
   
   // Send to worker
   int sent = SocketSend(socketHandle, data, ArraySize(data));
   
   if(sent <= 0)
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Failed to send acknowledgment, error: ", error);
      if(error == 5273) // ERR_NETSOCKET_IO_ERROR
      {
         Print("[RECEIVER] Connection lost (ERR_NETSOCKET_IO_ERROR). Will attempt reconnect.");
         g_connection_lost = true;
      }
   }
}
