//+------------------------------------------------------------------+
//|                                              signal_sender.mq5    |
//|                                         Trade Copier Master EA    |
//|                                    Sends trades to Rust Router    |
//+------------------------------------------------------------------+
#property copyright "Trade Copier"
#property version   "1.00"
#property strict

#define INVALID_SOCKET -1              // Invalid socket handle

input string RouterIP = "127.0.0.1";  // Rust Router IP
input int RouterPort = 5000;           // Rust Router Port

int socketHandle = INVALID_SOCKET;
bool g_connection_lost = false;
datetime g_last_send_time = 0;

// Position tracking: maps position ticket to trade_id
ulong g_position_tickets[];
ulong g_trade_ids[];
int g_tracking_count = 0;

ulong g_order_tickets[];
ulong g_order_trade_ids[];
string g_order_symbols[];
int g_order_tracking_count = 0;

// Position state tracking for modify detection
struct PositionState {
   double sl;
   double tp;
};
PositionState g_position_states[];

// Helper functions for position tracking
int FindPositionIndex(ulong ticket);
void AddPositionTracking(ulong ticket, ulong trade_id, double sl, double tp);
void RemovePositionTracking(ulong ticket);
void CheckPositionModifications();

// Helper functions for pending order tracking
int FindOrderIndex(ulong ticket);
void AddOrderTracking(ulong ticket, ulong trade_id, string symbol);
void RemoveOrderTracking(ulong ticket);
string GetOrderTypeString(ENUM_ORDER_TYPE order_type);

// Connection management
bool ConnectToRouter();
void DisconnectFromRouter();
bool EnsureConnection();

// Visual indicator
void DrawStatusIndicator();

//+------------------------------------------------------------------+
//| Expert initialization function                                   |
//+------------------------------------------------------------------+
int OnInit()
{
   // Initialize tracking arrays
   ArrayResize(g_position_tickets, 0);
   ArrayResize(g_trade_ids, 0);
   ArrayResize(g_position_states, 0);
   g_tracking_count = 0;
   
   ArrayResize(g_order_tickets, 0);
   ArrayResize(g_order_trade_ids, 0);
   ArrayResize(g_order_symbols, 0);
   g_order_tracking_count = 0;
   
   g_connection_lost = false;
   g_last_send_time = 0;
   
   Print("[SENDER] Trade Copier Master EA started");
   Print("[SENDER] Sending signals to ", RouterIP, ":", RouterPort);
   
   // Try initial connection, but don't fail if router is unavailable
   if(!ConnectToRouter())
   {
      Print("[SENDER] WARNING: Initial connection to router failed. Will retry automatically.");
      g_connection_lost = true;
   }
   
   // Draw initial status indicator
   DrawStatusIndicator();
   
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert tick function (for modify detection)                      |
//+------------------------------------------------------------------+
void OnTick()
{
   // Reconnect if connection lost
   if(g_connection_lost)
   {
      datetime current_time = TimeLocal();
      if(current_time - g_last_send_time > 5) // Try reconnect every 5 seconds
      {
         Print("[SENDER] Attempting to reconnect...");
         DisconnectFromRouter();
         g_connection_lost = !ConnectToRouter();
         g_last_send_time = current_time;
      }
   }
   
   CheckPositionModifications();
   
   // Update status indicator
   DrawStatusIndicator();
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                 |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   // Clean up status indicator
   ObjectDelete(0, "ProviderStatus");
   ObjectDelete(0, "ProviderLabel");
   
   DisconnectFromRouter();
   Print("[SENDER] Master EA stopped");
}

//+------------------------------------------------------------------+
//| Trade transaction event                                          |
//+------------------------------------------------------------------+
void OnTradeTransaction(
   const MqlTradeTransaction& trans,
   const MqlTradeRequest& request,
   const MqlTradeResult& result
)
{
   // Process pending order placement
   if(trans.type == TRADE_TRANSACTION_ORDER_ADD)
   {
      ulong order_ticket = trans.order;
      if(order_ticket > 0 && OrderSelect(order_ticket))
      {
         ENUM_ORDER_TYPE order_type = (ENUM_ORDER_TYPE)OrderGetInteger(ORDER_TYPE);
         
         // Only process pending orders (not market orders)
         if(order_type == ORDER_TYPE_BUY_LIMIT || order_type == ORDER_TYPE_SELL_LIMIT ||
            order_type == ORDER_TYPE_BUY_STOP || order_type == ORDER_TYPE_SELL_STOP)
         {
            string symbol = OrderGetString(ORDER_SYMBOL);
            double lots = OrderGetDouble(ORDER_VOLUME_CURRENT);
            double price = OrderGetDouble(ORDER_PRICE_OPEN);
            double sl = OrderGetDouble(ORDER_SL);
            double tp = OrderGetDouble(ORDER_TP);
            
            // Generate unique trade ID and track order
            ulong trade_id = (ulong)TimeLocal() * 1000000 + order_ticket;
            AddOrderTracking(order_ticket, trade_id, symbol);
            
            // Determine trade type (buy or sell)
            string trade_type = (order_type == ORDER_TYPE_BUY_LIMIT || order_type == ORDER_TYPE_BUY_STOP) ? "buy" : "sell";
            
            // Build and send pending order signal
            string json = "{\"id\":" + IntegerToString(trade_id) + 
                         ",\"symbol\":\"" + symbol + 
                         "\",\"type\":\"" + trade_type + 
                         "\",\"lots\":" + DoubleToString(lots, 2) + 
                         ",\"price\":" + DoubleToString(price, 5) + 
                         ",\"order_type\":\"" + GetOrderTypeString(order_type) + "\"";
            
            if(sl > 0) json += ",\"sl\":" + DoubleToString(sl, 5);
            if(tp > 0) json += ",\"tp\":" + DoubleToString(tp, 5);
            json += ",\"cmd\":\"open\"}";
            
            SendTradeSignal(json);
         }
      }
   }
   // Process pending order deletion (cancelled or expired)
   else if(trans.type == TRADE_TRANSACTION_ORDER_DELETE)
   {
      ulong order_ticket = trans.order;
      int idx = FindOrderIndex(order_ticket);
      
      if(idx >= 0)
      {
         // Check if order was filled (it will be in history with STATE_FILLED)
         // If filled, position tracking will handle it, so we just remove order tracking
         bool was_filled = false;
         if(HistoryOrderSelect(order_ticket))
         {
            ENUM_ORDER_STATE state = (ENUM_ORDER_STATE)HistoryOrderGetInteger(order_ticket, ORDER_STATE);
            was_filled = (state == ORDER_STATE_FILLED || state == ORDER_STATE_PARTIAL);
         }
         
         // Only send cancel signal if order was not filled (manually cancelled or expired)
         if(!was_filled)
         {
            string json = "{\"id\":" + IntegerToString(g_order_trade_ids[idx]) + 
                         ",\"symbol\":\"" + g_order_symbols[idx] + "\"" +
                         ",\"lots\":0" +
                         ",\"cmd\":\"cancel\"}";
            
            SendTradeSignal(json);
         }
         
         RemoveOrderTracking(order_ticket);
      }
   }
   // Process deal additions (position opened/closed)
   else if(trans.type == TRADE_TRANSACTION_DEAL_ADD)
   {
      ulong deal_ticket = trans.deal;
      if(deal_ticket > 0)
      {
         if(HistoryDealSelect(deal_ticket))
         {
            string symbol = HistoryDealGetString(deal_ticket, DEAL_SYMBOL);
            double lots = HistoryDealGetDouble(deal_ticket, DEAL_VOLUME);
            ENUM_DEAL_TYPE deal_type = (ENUM_DEAL_TYPE)HistoryDealGetInteger(deal_ticket, DEAL_TYPE);
            double price = HistoryDealGetDouble(deal_ticket, DEAL_PRICE);
            ENUM_DEAL_ENTRY deal_entry = (ENUM_DEAL_ENTRY)HistoryDealGetInteger(deal_ticket, DEAL_ENTRY);
            ulong position_ticket = HistoryDealGetInteger(deal_ticket, DEAL_POSITION_ID);
            
            // Handle position OPEN
            if(deal_entry == DEAL_ENTRY_IN && (deal_type == DEAL_TYPE_BUY || deal_type == DEAL_TYPE_SELL))
            {
               // Get position info for SL/TP
               double sl = 0.0, tp = 0.0;
               if(PositionSelectByTicket(position_ticket))
               {
                  sl = PositionGetDouble(POSITION_SL);
                  tp = PositionGetDouble(POSITION_TP);
               }
               
               // Generate unique trade ID and track position
               ulong trade_id = (ulong)TimeLocal() * 1000000 + deal_ticket;
               AddPositionTracking(position_ticket, trade_id, sl, tp);
               
               // Build and send open signal
               string json = "{\"id\":" + IntegerToString(trade_id) + 
                            ",\"symbol\":\"" + symbol + 
                            "\",\"type\":\"" + ((deal_type == DEAL_TYPE_BUY) ? "buy" : "sell") + 
                            "\",\"lots\":" + DoubleToString(lots, 2) + 
                            ",\"price\":" + DoubleToString(price, 5) + 
                            ",\"order_type\":\"market\"";
               
               if(sl > 0) json += ",\"sl\":" + DoubleToString(sl, 5);
               if(tp > 0) json += ",\"tp\":" + DoubleToString(tp, 5);
               json += ",\"cmd\":\"open\"}";
               
               SendTradeSignal(json);
            }
            // Handle position CLOSE (full or partial)
            else if(deal_entry == DEAL_ENTRY_OUT)
            {
               int idx = FindPositionIndex(position_ticket);
               if(idx >= 0)
               {
                  // Check if position still exists (partial close) or is fully closed
                  bool is_partial_close = PositionSelectByTicket(position_ticket);
                  string cmd = is_partial_close ? "partial_close" : "close";
                  
                  // Build and send close signal with actual closed volume
                  string json = "{\"id\":" + IntegerToString(g_trade_ids[idx]) + 
                               ",\"symbol\":\"" + symbol + 
                               "\",\"lots\":" + DoubleToString(lots, 2) + 
                               ",\"cmd\":\"" + cmd + "\"}";
                  
                  SendTradeSignal(json);
                  
                  // Only remove tracking if it's a full close
                  if(!is_partial_close)
                  {
                     RemovePositionTracking(position_ticket);
                  }
               }
            }
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Check for position modifications (SL/TP changes)                 |
//+------------------------------------------------------------------+
void CheckPositionModifications()
{
   for(int i = 0; i < g_tracking_count; i++)
   {
      if(!PositionSelectByTicket(g_position_tickets[i]))
         continue;
      
      double current_sl = PositionGetDouble(POSITION_SL);
      double current_tp = PositionGetDouble(POSITION_TP);
      
      // Check if SL or TP changed
      if(current_sl == g_position_states[i].sl && current_tp == g_position_states[i].tp)
         continue;
      
      // Update stored state
      g_position_states[i].sl = current_sl;
      g_position_states[i].tp = current_tp;
      
      // Build and send modify signal
      string json = "{\"id\":" + IntegerToString(g_trade_ids[i]) + 
                    ",\"symbol\":\"" + PositionGetString(POSITION_SYMBOL) + 
                    "\",\"type\":\"" + ((PositionGetInteger(POSITION_TYPE) == POSITION_TYPE_BUY) ? "buy" : "sell") + 
                    "\",\"lots\":" + DoubleToString(PositionGetDouble(POSITION_VOLUME), 2);
      
      if(current_sl > 0) json += ",\"sl\":" + DoubleToString(current_sl, 5);
      if(current_tp > 0) json += ",\"tp\":" + DoubleToString(current_tp, 5);
      json += ",\"cmd\":\"modify\"}";
      
      SendTradeSignal(json);
   }
}

//+------------------------------------------------------------------+
//| Send trade signal via TCP                                        |
//+------------------------------------------------------------------+
void SendTradeSignal(string json)
{
   if(!EnsureConnection())
   {
      Print("[SENDER] ERROR: Cannot send signal, no connection to router");
      return;
   }
   
   Print("[SENDER] Sending: ", json);
   
   json += "\n";
   uchar data[];
   int len = StringToCharArray(json, data, 0, WHOLE_ARRAY, CP_UTF8) - 1;
   ArrayResize(data, len);
   
   int sent = SocketSend(socketHandle, data, len);
   if(sent <= 0)
   {
      int error = GetLastError();
      Print("[SENDER] ERROR: Send failed, error: ", error);
      if(error == 5273) // ERR_NETSOCKET_IO_ERROR
      {
         Print("[SENDER] Connection lost (ERR_NETSOCKET_IO_ERROR). Will attempt reconnect.");
         g_connection_lost = true;
      }
   }
   else
   {
      g_last_send_time = TimeLocal();
   }
}

//+------------------------------------------------------------------+
//| Position tracking helper functions                               |
//+------------------------------------------------------------------+
int FindPositionIndex(ulong ticket)
{
   for(int i = 0; i < g_tracking_count; i++)
   {
      if(g_position_tickets[i] == ticket)
         return i;
   }
   return -1;
}

void AddPositionTracking(ulong ticket, ulong trade_id, double sl, double tp)
{
   g_tracking_count++;
   ArrayResize(g_position_tickets, g_tracking_count);
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_states, g_tracking_count);
   
   g_position_tickets[g_tracking_count - 1] = ticket;
   g_trade_ids[g_tracking_count - 1] = trade_id;
   g_position_states[g_tracking_count - 1].sl = sl;
   g_position_states[g_tracking_count - 1].tp = tp;
   
   Print("[SENDER] Tracking position: ticket=", ticket, " trade_id=", trade_id);
}

void RemovePositionTracking(ulong ticket)
{
   int idx = FindPositionIndex(ticket);
   if(idx < 0) return;
   
   g_tracking_count--;
   
   // Shift arrays to remove element
   for(int i = idx; i < g_tracking_count; i++)
   {
      g_position_tickets[i] = g_position_tickets[i + 1];
      g_trade_ids[i] = g_trade_ids[i + 1];
      g_position_states[i] = g_position_states[i + 1];
   }
   
   ArrayResize(g_position_tickets, g_tracking_count);
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_states, g_tracking_count);
}

//+------------------------------------------------------------------+
//| Connection management functions                                  |
//+------------------------------------------------------------------+
bool ConnectToRouter()
{
   // Initialize TCP socket
   socketHandle = SocketCreate();
   if(socketHandle == INVALID_SOCKET)
   {
      int error = GetLastError();
      Print("[SENDER] ERROR: Failed to create socket, error code: ", error);
      return false;
   }
   
   Print("[SENDER] Socket created successfully, handle: ", socketHandle);
   
   // Connect to router
   if(!SocketConnect(socketHandle, RouterIP, RouterPort, 1000))
   {
      int error = GetLastError();
      Print("[SENDER] ERROR: Failed to connect to router at ", RouterIP, ":", RouterPort, ", error code: ", error);
      Print("[SENDER] Common error codes: 5002=DLL not allowed, 4014=Internal error, 5200=Socket error");
      SocketClose(socketHandle);
      socketHandle = INVALID_SOCKET;
      return false;
   }
   
   Print("[SENDER] Connected to router successfully (TCP)");
   g_connection_lost = false;
   g_last_send_time = TimeLocal();
   return true;
}

void DisconnectFromRouter()
{
   if(socketHandle != INVALID_SOCKET)
   {
      SocketClose(socketHandle);
      socketHandle = INVALID_SOCKET;
      Print("[SENDER] Disconnected from router");
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
//| Pending order tracking helper functions                         |
//+------------------------------------------------------------------+
int FindOrderIndex(ulong ticket)
{
   for(int i = 0; i < g_order_tracking_count; i++)
   {
      if(g_order_tickets[i] == ticket)
         return i;
   }
   return -1;
}

void AddOrderTracking(ulong ticket, ulong trade_id, string symbol)
{
   g_order_tracking_count++;
   ArrayResize(g_order_tickets, g_order_tracking_count);
   ArrayResize(g_order_trade_ids, g_order_tracking_count);
   ArrayResize(g_order_symbols, g_order_tracking_count);
   
   g_order_tickets[g_order_tracking_count - 1] = ticket;
   g_order_trade_ids[g_order_tracking_count - 1] = trade_id;
   g_order_symbols[g_order_tracking_count - 1] = symbol;
   
   Print("[SENDER] Tracking pending order: ticket=", ticket, " trade_id=", trade_id, " symbol=", symbol);
}

void RemoveOrderTracking(ulong ticket)
{
   int idx = FindOrderIndex(ticket);
   if(idx < 0) return;
   
   g_order_tracking_count--;
   
   // Shift arrays to remove element
   for(int i = idx; i < g_order_tracking_count; i++)
   {
      g_order_tickets[i] = g_order_tickets[i + 1];
      g_order_trade_ids[i] = g_order_trade_ids[i + 1];
      g_order_symbols[i] = g_order_symbols[i + 1];
   }
   
   ArrayResize(g_order_tickets, g_order_tracking_count);
   ArrayResize(g_order_trade_ids, g_order_tracking_count);
   ArrayResize(g_order_symbols, g_order_tracking_count);
}

string GetOrderTypeString(ENUM_ORDER_TYPE order_type)
{
   switch(order_type)
   {
      case ORDER_TYPE_BUY_LIMIT:  return "buy_limit";
      case ORDER_TYPE_SELL_LIMIT: return "sell_limit";
      case ORDER_TYPE_BUY_STOP:   return "buy_stop";
      case ORDER_TYPE_SELL_STOP:  return "sell_stop";
      default: return "market";
   }
}

//+------------------------------------------------------------------+
//| Draw status indicator on chart                                   |
//+------------------------------------------------------------------+
void DrawStatusIndicator()
{
   string labelName = "ProviderStatus";
   string textName = "ProviderLabel";
   
   // Determine color based on connection status
   color statusColor = g_connection_lost ? clrRed : clrLimeGreen;
   string statusText = g_connection_lost ? "Provider: DISCONNECTED" : "Provider: CONNECTED";
   
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
