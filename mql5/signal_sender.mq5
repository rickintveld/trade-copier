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
   
   Print("[SENDER] Trade Copier Master EA started");
   Print("[SENDER] Sending signals to ", RouterIP, ":", RouterPort);
   
   // Initialize TCP socket
   socketHandle = SocketCreate();
   if(socketHandle == INVALID_SOCKET)
   {
      int error = GetLastError();
      Print("[SENDER] ERROR: Failed to create socket, error code: ", error);
      return INIT_FAILED;
   }
   
   Print("[SENDER] Socket created successfully, handle: ", socketHandle);
   
   // Connect to router
   if(!SocketConnect(socketHandle, RouterIP, RouterPort, 1000))
   {
      int error = GetLastError();
      Print("[SENDER] ERROR: Failed to connect to router at ", RouterIP, ":", RouterPort, ", error code: ", error);
      Print("[SENDER] Common error codes: 5002=DLL not allowed, 4014=Internal error, 5200=Socket error");
      SocketClose(socketHandle);
      return INIT_FAILED;
   }
   
   Print("[SENDER] Connected to router successfully (TCP)");
   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| Expert tick function (for modify detection)                      |
//+------------------------------------------------------------------+
void OnTick()
{
   CheckPositionModifications();
}

//+------------------------------------------------------------------+
//| Expert deinitialization function                                 |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   if(socketHandle != INVALID_SOCKET)
      SocketClose(socketHandle);
   
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
            
            // Determine if this is an entry (open) or exit (close)
            if(deal_entry == DEAL_ENTRY_IN)
            {
               // Position OPEN
               if(deal_type == DEAL_TYPE_BUY || deal_type == DEAL_TYPE_SELL)
               {
                  string trade_type = (deal_type == DEAL_TYPE_BUY) ? "buy" : "sell";
                  
                  // Get position info for SL/TP
                  double sl = 0.0;
                  double tp = 0.0;
                  if(PositionSelectByTicket(position_ticket))
                  {
                     sl = PositionGetDouble(POSITION_SL);
                     tp = PositionGetDouble(POSITION_TP);
                  }
                  
                  // Generate unique trade ID
                  ulong trade_id = (ulong)TimeLocal() * 1000000 + deal_ticket;
                  
                  // Track this position
                  AddPositionTracking(position_ticket, trade_id, sl, tp);
                  
                  // Build and send open signal
                  string json = "{";
                  json += "\"id\":" + IntegerToString(trade_id) + ",";
                  json += "\"symbol\":\"" + symbol + "\",";
                  json += "\"type\":\"" + trade_type + "\",";
                  json += "\"lots\":" + DoubleToString(lots, 2) + ",";
                  json += "\"price\":" + DoubleToString(price, 5);
                  if(sl > 0) json += ",\"sl\":" + DoubleToString(sl, 5);
                  if(tp > 0) json += ",\"tp\":" + DoubleToString(tp, 5);
                  json += ",\"cmd\":\"open\"}";
                  
                  SendTradeSignal(json);
               }
            }
            else if(deal_entry == DEAL_ENTRY_OUT)
            {
               // Position CLOSE
               int idx = FindPositionIndex(position_ticket);
               if(idx >= 0)
               {
                  ulong trade_id = g_trade_ids[idx];
                  
                  // Build and send close signal
                  string json = "{";
                  json += "\"id\":" + IntegerToString(trade_id) + ",";
                  json += "\"symbol\":\"" + symbol + "\",";
                  json += "\"type\":\"buy\",";  // type not critical for close
                  json += "\"lots\":" + DoubleToString(lots, 2) + ",";
                  json += "\"cmd\":\"close\"}";
                  
                  SendTradeSignal(json);
                  
                  // Remove from tracking
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
      ulong ticket = g_position_tickets[i];
      
      if(PositionSelectByTicket(ticket))
      {
         double current_sl = PositionGetDouble(POSITION_SL);
         double current_tp = PositionGetDouble(POSITION_TP);
         
         // Check if SL or TP changed
         if(current_sl != g_position_states[i].sl || current_tp != g_position_states[i].tp)
         {
            // Update stored state
            g_position_states[i].sl = current_sl;
            g_position_states[i].tp = current_tp;
            
            // Send modify signal
            string symbol = PositionGetString(POSITION_SYMBOL);
            string type = (PositionGetInteger(POSITION_TYPE) == POSITION_TYPE_BUY) ? "buy" : "sell";
            double lots = PositionGetDouble(POSITION_VOLUME);
            
            string json = "{";
            json += "\"id\":" + IntegerToString(g_trade_ids[i]) + ",";
            json += "\"symbol\":\"" + symbol + "\",";
            json += "\"type\":\"" + type + "\",";
            json += "\"lots\":" + DoubleToString(lots, 2);
            if(current_sl > 0) json += ",\"sl\":" + DoubleToString(current_sl, 5);
            if(current_tp > 0) json += ",\"tp\":" + DoubleToString(current_tp, 5);
            json += ",\"cmd\":\"modify\"}";
            
            SendTradeSignal(json);
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Send trade signal via TCP                                        |
//+------------------------------------------------------------------+
void SendTradeSignal(string json)
{
   Print("[SENDER] Sending trade: ", json);
   
   // Add newline delimiter for message framing
   json += "\n";
   
   // Convert string to char array for socket
   uchar data[];
   StringToCharArray(json, data, 0, StringLen(json));
   
   // Send TCP packet
   int sent = SocketSend(socketHandle, data, ArraySize(data));
   
   if(sent > 0)
      Print("[SENDER] Trade signal sent successfully (", sent, " bytes)");
   else
      Print("[SENDER] ERROR: Failed to send trade signal, error: ", GetLastError());
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
   
   // Shift arrays to remove element
   for(int i = idx; i < g_tracking_count - 1; i++)
   {
      g_position_tickets[i] = g_position_tickets[i + 1];
      g_trade_ids[i] = g_trade_ids[i + 1];
      g_position_states[i] = g_position_states[i + 1];
   }
   
   g_tracking_count--;
   ArrayResize(g_position_tickets, g_tracking_count);
   ArrayResize(g_trade_ids, g_tracking_count);
   ArrayResize(g_position_states, g_tracking_count);
   
   Print("[SENDER] Stopped tracking position: ticket=", ticket);
}
