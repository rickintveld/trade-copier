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
string g_receive_buffer = "";  // Buffer for incomplete messages

// Position tracking: maps trade_id to position ticket
ulong g_trade_ids[];
ulong g_position_tickets[];
int g_tracking_count = 0;

ulong g_order_trade_ids[];
ulong g_order_tickets[];
int g_order_tracking_count = 0;

// Helper functions
int FindTradeIdIndex(ulong trade_id);
void AddPositionMapping(ulong trade_id, ulong ticket);
void RemovePositionMapping(ulong trade_id);
void SendAcknowledgment(bool success, string message);
void SendProfit(double profit);

// Order tracking helper functions
int FindOrderTradeIdIndex(ulong trade_id);
void AddOrderMapping(ulong trade_id, ulong ticket);
void RemoveOrderMapping(ulong trade_id);

// Connection management
bool ConnectToWorker();
void DisconnectFromWorker();
bool EnsureConnection();

// Visual indicator
void DrawStatusIndicator();

//+------------------------------------------------------------------+
//| Expert initialization function                                   |
//+------------------------------------------------------------------+
int OnInit()
{
   // Initialize tracking arrays
   ArrayResize(g_trade_ids, 0);
   ArrayResize(g_position_tickets, 0);
   g_tracking_count = 0;
   
   ArrayResize(g_order_trade_ids, 0);
   ArrayResize(g_order_tickets, 0);
   g_order_tracking_count = 0;
   
   g_connection_lost = false;
   g_last_recv_time = 0;
   
   Print("[RECEIVER] Trade Copier Slave EA started");
   Print("[RECEIVER] Connecting to worker at ", WorkerIP, ":", WorkerPort);

   // Set trade parameters
   trade.SetExpertMagicNumber(MagicNumber);
   trade.SetDeviationInPoints(Slippage);
   trade.SetTypeFilling(ORDER_FILLING_FOK);

   // Try initial connection, but don't fail if worker is unavailable
   if(!ConnectToWorker())
   {
      Print("[RECEIVER] WARNING: Initial connection to worker failed. Will retry automatically.");
      g_connection_lost = true;
   }

   // Draw initial status indicator
   DrawStatusIndicator();

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                 |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Clean up status indicator
   ObjectDelete(0, "ReceiverStatus");
   ObjectDelete(0, "ReceiverLabel");
   
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
      if(current_time - g_last_recv_time > 1) // Try reconnect every 1 second (faster recovery)
      {
         Print("[RECEIVER] Attempting to reconnect...");
         DisconnectFromWorker();
         g_connection_lost = !ConnectToWorker();
         g_last_recv_time = current_time;
      }
   }
   // Check for incoming TCP messages
   CheckIncomingTrades();
   
   // Update status indicator
   DrawStatusIndicator();
}

//+------------------------------------------------------------------+
//| Trade transaction event handler                                  |
//+------------------------------------------------------------------+
void OnTradeTransaction(const MqlTradeTransaction &trans,
                        const MqlTradeRequest &request,
                        const MqlTradeResult &result)
{
   // Check if a deal was completed (position closed or modified)
   if(trans.type == TRADE_TRANSACTION_DEAL_ADD)
   {
      // Get deal information
      if(HistoryDealSelect(trans.deal))
      {
         ENUM_DEAL_ENTRY entry = (ENUM_DEAL_ENTRY)HistoryDealGetInteger(trans.deal, DEAL_ENTRY);
         
         // If this is an exit deal (position closed), send profit
         if(entry == DEAL_ENTRY_OUT)
         {
            // Get the profit from the closed deal (includes commission and swap)
            double dealProfit = HistoryDealGetDouble(trans.deal, DEAL_PROFIT);
            double dealCommission = HistoryDealGetDouble(trans.deal, DEAL_COMMISSION);
            double dealSwap = HistoryDealGetDouble(trans.deal, DEAL_SWAP);
            double totalProfit = dealProfit + dealCommission + dealSwap;
            
            Print("[RECEIVER] Position closed - Profit: ", dealProfit, ", Commission: ", dealCommission, ", Swap: ", dealSwap, ", Total: ", totalProfit);
            SendProfit(totalProfit);
         }
      }
   }
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

      // Convert to string and append to buffer
      string data = CharArrayToString(buffer, 0, received, CP_UTF8);
      g_receive_buffer += data;

      // Process complete messages (newline-delimited)
      int newline_pos;
      while((newline_pos = StringFind(g_receive_buffer, "\n")) >= 0)
      {
         // Extract complete message
         string message = StringSubstr(g_receive_buffer, 0, newline_pos);
         
         // Remove processed message from buffer (including newline)
         g_receive_buffer = StringSubstr(g_receive_buffer, newline_pos + 1);
         
         // Process message if not empty
         if(StringLen(message) > 0)
         {
            // Check for heartbeat PING message
            if(message == "PING")
            {
               // Heartbeat from worker - connection is alive, no action needed
            }
            else
            {
               ParseAndExecuteTrade(message);
            }
         }
      }
   }
   else if(received < 0)
   {
      int error = GetLastError();

      Print("[RECEIVER] ERROR: Read failed, error: ", error);
      if(error == 5273 || error == 5274 || error == 4014) // Network errors
      {
         Print("[RECEIVER] Connection lost (ERR_NETSOCKET_IO_ERROR). Will attempt reconnect.");
         g_connection_lost = true;
         g_last_recv_time = 0; // Trigger immediate reconnection attempt
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
   string order_type = "market";

   // Extract fields from JSON
   if(!ExtractJSONULong(json_data, "id", trade_id)) 
   {
      Print("[RECEIVER] ERROR: Failed to parse trade_id");
      SendAcknowledgment(false, "Failed to parse trade_id");
      return false;
   }

   if(!ExtractJSONString(json_data, "symbol", symbol)) 
   {
      Print("[RECEIVER] ERROR: Failed to parse symbol");
      SendAcknowledgment(false, "Failed to parse symbol");
      return false;
   }

   if(!ExtractJSONDouble(json_data, "lots", lots)) 
   {
      Print("[RECEIVER] ERROR: Failed to parse lots");
      SendAcknowledgment(false, "Failed to parse lots");
      return false;
   }

   // Optional fields
   ExtractJSONString(json_data, "type", trade_type);
   ExtractJSONDouble(json_data, "price", price);
   ExtractJSONDouble(json_data, "sl", sl);
   ExtractJSONDouble(json_data, "tp", tp);
   ExtractJSONString(json_data, "cmd", cmd);
   ExtractJSONString(json_data, "order_type", order_type);
   
   // Handle different commands
   bool success = false;

   if(cmd == "open")
   {
      Print("[RECEIVER] Opening ", trade_type, " ", lots, " lots ", symbol, " (ID: ", trade_id, ")");
      
      // Validate trade_type for open command
      if(trade_type == "")
      {
         Print("[RECEIVER] ERROR: trade_type required for open command");
         SendAcknowledgment(false, "trade_type required for open command");
         return false;
      }

      ulong ticket = 0;
      
      // Check if this is a pending order or market order
      if(order_type == "market")
      {
         // Execute market order
         if(trade_type == "buy")
         {
            success = trade.Buy(lots, symbol, 0, sl, tp, "CopiedTrade");
            if(!success)
            {
               Print("[RECEIVER] ERROR: BUY failed - ", trade.ResultRetcodeDescription());
            }
         }
         else if(trade_type == "sell")
         {
            success = trade.Sell(lots, symbol, 0, sl, tp, "CopiedTrade");
            if(!success)
            {
               Print("[RECEIVER] ERROR: SELL failed - ", trade.ResultRetcodeDescription());
            }
         }
         else
         {
            Print("[RECEIVER] ERROR: Invalid trade_type: ", trade_type);
            SendAcknowledgment(false, "Invalid trade_type: " + trade_type);
            return false;
         }
         
        // Handle ticket result for market orders
        if(success)
        {
            // Select position directly by symbol
            if(PositionSelect(symbol))
            {
                ticket = PositionGetInteger(POSITION_TICKET);
                
                if(ticket > 0)
                {
                    Print("[RECEIVER] Position opened: ticket=", ticket);
                    AddPositionMapping(trade_id, ticket);
                    SendAcknowledgment(true, "Market order opened successfully");
                }
                else
                {
                    Print("[RECEIVER] ERROR: Invalid position ticket");
                    SendAcknowledgment(false, "Got invalid position ticket");
                }
            }
            else
            {
                Print("[RECEIVER] WARNING: Position not found after opening (timing issue, will auto-discover on modify)");
                SendAcknowledgment(true, "Position opened but not immediately available");
            }
        }
         else
         {
            SendAcknowledgment(false, "Failed to open market order");
         }
      }
      else
      {
         // Place pending order
         if(price <= 0)
         {
            Print("[RECEIVER] ERROR: Price required for pending orders");
            SendAcknowledgment(false, "Price required for pending orders");
            return false;
         }
         
         if(order_type == "buy_limit")
         {
            success = trade.BuyLimit(lots, price, symbol, sl, tp, ORDER_TIME_GTC, 0, "CopiedOrder");
         }
         else if(order_type == "sell_limit")
         {
            success = trade.SellLimit(lots, price, symbol, sl, tp, ORDER_TIME_GTC, 0, "CopiedOrder");
         }
         else if(order_type == "buy_stop")
         {
            success = trade.BuyStop(lots, price, symbol, sl, tp, ORDER_TIME_GTC, 0, "CopiedOrder");
         }
         else if(order_type == "sell_stop")
         {
            success = trade.SellStop(lots, price, symbol, sl, tp, ORDER_TIME_GTC, 0, "CopiedOrder");
         }
         else
         {
            SendAcknowledgment(false, "Invalid order_type: " + order_type);
            return false;
         }
         
         if(success)
         {
            ticket = trade.ResultOrder();
            
            if(ticket > 0)
            {
               AddOrderMapping(trade_id, ticket);
               SendAcknowledgment(true, "Pending order placed successfully");
            }
            else
            {
               SendAcknowledgment(false, "Failed to get order ticket");
            }
         }
         else
         {
            SendAcknowledgment(false, "Failed to place pending order");
         }
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
         // Note: Profit is sent via OnTradeTransaction when deal completes
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
         // Send updated account info after partial clos
      }
      else
      {
         SendAcknowledgment(false, "Failed to partially close trade");
      }

      return success;
   }
   else if(cmd == "modify")
   {
      Print("[RECEIVER] Modifying position: SL=", sl, " TP=", tp, " (ID: ", trade_id, ")");
      
      int idx = FindTradeIdIndex(trade_id);

      if(idx < 0) 
      {
         // Try to auto-discover position by symbol
         if(PositionSelect(symbol))
         {
            ulong ticket = PositionGetInteger(POSITION_TICKET);
            Print("[RECEIVER] Auto-discovered position: ticket=", ticket);
            
            AddPositionMapping(trade_id, ticket);
            idx = FindTradeIdIndex(trade_id);
            
            if(idx < 0)
            {
               Print("[RECEIVER] ERROR: Failed to add position to tracking");
               SendAcknowledgment(false, "Failed to add position to tracking");
               return false;
            }
         }
         else
         {
            Print("[RECEIVER] ERROR: Position not found for ", symbol);
            SendAcknowledgment(false, "Position not found");
            return false;
         }
      }

      ulong ticket = g_position_tickets[idx];

      if(!PositionSelectByTicket(ticket))
      {
         Print("[RECEIVER] ERROR: Position not found with ticket ", ticket);
         RemovePositionMapping(trade_id);
         SendAcknowledgment(false, "Position not found");
         return false;
      }

      success = trade.PositionModify(ticket, sl, tp);
      if(success)
      {
         Print("[RECEIVER] Position modified: ticket=", ticket);
         SendAcknowledgment(true, "Trade modified successfully");
      }
      else
      {
         Print("[RECEIVER] ERROR: Modify failed - ", trade.ResultRetcodeDescription());
         SendAcknowledgment(false, "Failed to modify trade");
      }

      return success;
   }
   else if(cmd == "cancel")
   {
      int idx = FindOrderTradeIdIndex(trade_id);
      
      if(idx < 0)
      {
         SendAcknowledgment(false, "Order ID not found");
         return false;
      }
      
      ulong order_ticket = g_order_tickets[idx];
      
      success = trade.OrderDelete(order_ticket);
      if(success)
      {
         RemoveOrderMapping(trade_id);
         SendAcknowledgment(true, "Order cancelled successfully");
      }
      else
      {
         SendAcknowledgment(false, "Failed to cancel order");
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
      g_receive_buffer = "";  // Clear receive buffer on disconnect
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
   
   // Verify socket is actually writable (not just non-null)
   if(!SocketIsWritable(socketHandle))
   {
      Print("[RECEIVER] Socket not writable, marking connection as lost");
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
      if(error == 5273 || error == 5274 || error == 4014) // Network errors
      {
         Print("[RECEIVER] Connection lost (ERR_NETSOCKET_IO_ERROR). Will attempt reconnect.");
         g_connection_lost = true;
         g_last_recv_time = 0; // Trigger immediate reconnection attempt
      }
   }
}

//+------------------------------------------------------------------+
//| Order tracking helper functions                                 |
//+------------------------------------------------------------------+
int FindOrderTradeIdIndex(ulong trade_id)
{
   for(int i = 0; i < g_order_tracking_count; i++)
   {
      if(g_order_trade_ids[i] == trade_id)
         return i;
   }
   return -1;
}

void AddOrderMapping(ulong trade_id, ulong ticket)
{
   int idx = FindOrderTradeIdIndex(trade_id);
   
   if(idx >= 0)
   {
      g_order_tickets[idx] = ticket;
      return;
   }
   
   g_order_tracking_count++;
   ArrayResize(g_order_trade_ids, g_order_tracking_count);
   ArrayResize(g_order_tickets, g_order_tracking_count);
   g_order_trade_ids[g_order_tracking_count - 1] = trade_id;
   g_order_tickets[g_order_tracking_count - 1] = ticket;
}

void RemoveOrderMapping(ulong trade_id)
{
   int idx = FindOrderTradeIdIndex(trade_id);
   
   if(idx < 0) return;
   
   for(int i = idx; i < g_order_tracking_count - 1; i++)
   {
      g_order_trade_ids[i] = g_order_trade_ids[i + 1];
      g_order_tickets[i] = g_order_tickets[i + 1];
   }
   
   g_order_tracking_count--;
   
   ArrayResize(g_order_trade_ids, g_order_tracking_count);
   ArrayResize(g_order_tickets, g_order_tracking_count);
}

//+------------------------------------------------------------------+
//| Send profit information to Rust worker                           |
//+------------------------------------------------------------------+
void SendProfit(double profit)
{
   if(!EnsureConnection())
      return;
   
   // Format as JSON
   string json = StringFormat("{\"profit\":%.2f}\n", profit);
   
   // Convert to bytes
   uchar data[];
   StringToCharArray(json, data, 0, StringLen(json), CP_UTF8);
   
   // Send to worker
   int sent = SocketSend(socketHandle, data, ArraySize(data));
   
   if(sent > 0)
   {
      Print("[RECEIVER] Profit sent: ", profit);
   }
   else
   {
      int error = GetLastError();
      Print("[RECEIVER] ERROR: Failed to send profit, error: ", error);
      if(error == 5273 || error == 5274 || error == 4014) // Network errors
      {
         Print("[RECEIVER] Connection lost. Will attempt reconnect.");
         g_connection_lost = true;
         g_last_recv_time = 0;
      }
   }
}

//+------------------------------------------------------------------+
//| Draw status indicator on chart                                   |
//+------------------------------------------------------------------+
void DrawStatusIndicator()
{
   string labelName = "ReceiverStatus";
   string textName = "ReceiverLabel";
   
   // Determine color based on connection status
   color statusColor = g_connection_lost ? clrRed : clrLimeGreen;
   string statusText = g_connection_lost ? "Receiver: DISCONNECTED" : "Receiver: CONNECTED";
   
   // Create or update status box
   if(ObjectFind(0, labelName) < 0)
   {
      ObjectCreate(0, labelName, OBJ_RECTANGLE_LABEL, 0, 0, 0);
      ObjectSetInteger(0, labelName, OBJPROP_CORNER, CORNER_LEFT_UPPER);
      ObjectSetInteger(0, labelName, OBJPROP_XDISTANCE, 10);
      ObjectSetInteger(0, labelName, OBJPROP_YDISTANCE, 30);
      ObjectSetInteger(0, labelName, OBJPROP_XSIZE, 200);
      ObjectSetInteger(0, labelName, OBJPROP_YSIZE, 30);
      ObjectSetInteger(0, labelName, OBJPROP_BORDER_TYPE, BORDER_FLAT);
      ObjectSetInteger(0, labelName, OBJPROP_WIDTH, 1);
      ObjectSetInteger(0, labelName, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, labelName, OBJPROP_HIDDEN, true);
   }
   
   ObjectSetInteger(0, labelName, OBJPROP_BGCOLOR, statusColor);
   ObjectSetInteger(0, labelName, OBJPROP_BORDER_COLOR, statusColor);
   
   // Create or update status text
   if(ObjectFind(0, textName) < 0)
   {
      ObjectCreate(0, textName, OBJ_LABEL, 0, 0, 0);
      ObjectSetInteger(0, textName, OBJPROP_CORNER, CORNER_LEFT_UPPER);
      ObjectSetInteger(0, textName, OBJPROP_XDISTANCE, 20);
      ObjectSetInteger(0, textName, OBJPROP_YDISTANCE, 38);
      ObjectSetInteger(0, textName, OBJPROP_FONTSIZE, 9);
      ObjectSetString(0, textName, OBJPROP_FONT, "Arial Bold");
      ObjectSetInteger(0, textName, OBJPROP_SELECTABLE, false);
      ObjectSetInteger(0, textName, OBJPROP_HIDDEN, true);
   }
   
   ObjectSetString(0, textName, OBJPROP_TEXT, statusText);
   ObjectSetInteger(0, textName, OBJPROP_COLOR, clrWhite);
   
   ChartRedraw();
}
