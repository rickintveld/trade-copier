//+------------------------------------------------------------------+
//|                                              signal_sender.mq5    |
//|                                         Trade Copier Master EA    |
//|                                    Sends trades to Rust Router    |
//+------------------------------------------------------------------+
#property copyright "Trade Copier"
#property version   "1.00"
#property strict

input string RouterIP = "127.0.0.1";  // Rust Router IP
input int RouterPort = 5000;           // Rust Router Port

int socketHandle = INVALID_SOCKET;

//+------------------------------------------------------------------+
//| Expert initialization function                                   |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("[SENDER] Trade Copier Master EA started");
   Print("[SENDER] Sending signals to ", RouterIP, ":", RouterPort);
   
   // Initialize UDP socket
   socketHandle = SocketCreate(SOCKET_UDP);
   if(socketHandle == INVALID_SOCKET)
   {
      Print("[SENDER] ERROR: Failed to create socket");
      return INIT_FAILED;
   }
   
   // Connect to router
   if(!SocketConnect(socketHandle, RouterIP, RouterPort, 1000))
   {
      Print("[SENDER] ERROR: Failed to connect to router at ", RouterIP, ":", RouterPort);
      SocketClose(socketHandle);
      return INIT_FAILED;
   }
   
   Print("[SENDER] Connected to router successfully");
   return INIT_SUCCEEDED;
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
   // Only process when a deal is added (position opened/closed)
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
            
            // Convert deal type to trade type
            string trade_type = "";
            if(deal_type == DEAL_TYPE_BUY)
               trade_type = "buy";
            else if(deal_type == DEAL_TYPE_SELL)
               trade_type = "sell";
            else
               return; // Ignore other deal types
            
            // Get position info for SL/TP
            double sl = 0.0;
            double tp = 0.0;
            if(PositionSelect(symbol))
            {
               sl = PositionGetDouble(POSITION_SL);
               tp = PositionGetDouble(POSITION_TP);
            }
            
            // Generate unique trade ID (using timestamp + ticket)
            ulong trade_id = (ulong)TimeLocal() * 1000000 + deal_ticket;
            
            // Build JSON message
            string json = BuildTradeJSON(trade_id, symbol, trade_type, lots, price, sl, tp);
            
            // Send via UDP
            SendTradeSignal(json);
         }
      }
   }
}

//+------------------------------------------------------------------+
//| Build JSON trade message                                         |
//+------------------------------------------------------------------+
string BuildTradeJSON(
   ulong trade_id,
   string symbol,
   string trade_type,
   double lots,
   double price,
   double sl,
   double tp
)
{
   string json = "{";
   json += "\"id\":" + IntegerToString(trade_id) + ",";
   json += "\"symbol\":\"" + symbol + "\",";
   json += "\"type\":\"" + trade_type + "\",";
   json += "\"lots\":" + DoubleToString(lots, 2) + ",";
   json += "\"price\":" + DoubleToString(price, 5);
   
   if(sl > 0)
      json += ",\"sl\":" + DoubleToString(sl, 5);
   if(tp > 0)
      json += ",\"tp\":" + DoubleToString(tp, 5);
   
   json += ",\"cmd\":\"open\"";
   json += "}";
   
   return json;
}

//+------------------------------------------------------------------+
//| Send trade signal via UDP                                        |
//+------------------------------------------------------------------+
void SendTradeSignal(string json)
{
   Print("[SENDER] Sending trade: ", json);
   
   // Convert string to char array for socket
   uchar data[];
   StringToCharArray(json, data, 0, StringLen(json));
   
   // Send UDP packet
   int sent = SocketSend(socketHandle, data, ArraySize(data));
   
   if(sent > 0)
      Print("[SENDER] Trade signal sent successfully (", sent, " bytes)");
   else
      Print("[SENDER] ERROR: Failed to send trade signal, error: ", GetLastError());
}
