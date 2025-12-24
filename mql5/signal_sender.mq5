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

// Connection management
bool ConnectToRouter();
void DisconnectFromRouter();
bool EnsureConnection();

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
   g_connection_lost = false;
   g_last_send_time = 0;
   
   Print("[SENDER] Trade Copier Master EA started");
   Print("[SENDER] Sending signals to ", RouterIP, ":", RouterPort);
   
   if(!ConnectToRouter())
      return INIT_FAILED;
   
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
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                 |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
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
   // Process deal additions (position opened/closed)
   if(trans.type == TRADE_TRANSACTION_DEAL_ADD)
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
                            ",\"price\":" + DoubleToString(price, 5);
               
               if(sl > 0) json += ",\"sl\":" + DoubleToString(sl, 5);
               if(tp > 0) json += ",\"tp\":" + DoubleToString(tp, 5);
               json += ",\"cmd\":\"open\"}";
               
               SendTradeSignal(json);
            }
            // Handle position CLOSE
            else if(deal_entry == DEAL_ENTRY_OUT)
            {
               int idx = FindPositionIndex(position_ticket);
               if(idx >= 0)
               {
                  // Build and send close signal
                  string json = "{\"id\":" + IntegerToString(g_trade_ids[idx]) + 
                               ",\"symbol\":\"" + symbol + 
                               "\",\"lots\":" + DoubleToString(lots, 2) + 
                               ",\"cmd\":\"close\"}";
                  
                  SendTradeSignal(json);
                  RemovePositionTracking(position_ticket);
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
