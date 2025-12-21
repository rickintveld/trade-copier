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
#define SOCKET_UDP 1                   // UDP socket type

input int ListenPort = 5050;           // UDP Listen Port
input int MagicNumber = 999888;        // Magic Number for trades
input int Slippage = 10;               // Slippage in points

int socketHandle = INVALID_SOCKET;
CTrade trade;

//+------------------------------------------------------------------+
//| Expert initialization function                                   |
//+------------------------------------------------------------------+
int OnInit()
{
   Print("[RECEIVER] Trade Copier Slave EA started");
   Print("[RECEIVER] Listening on port ", ListenPort);
   
   // Initialize UDP socket
   // Note: MQL5 doesn't support server-side UDP listening with SocketBind
   // For receiving UDP packets, you'll need to use a different approach
   // such as connecting to a local address or using TCP sockets instead
   socketHandle = SocketCreate(SOCKET_UDP);
   if(socketHandle == INVALID_SOCKET)
   {
      Print("[RECEIVER] ERROR: Failed to create socket");
      return INIT_FAILED;
   }
   
   // Connect to localhost to receive on specific port
   if(!SocketConnect(socketHandle, "127.0.0.1", ListenPort, 1000))
   {
      Print("[RECEIVER] ERROR: Failed to connect to port ", ListenPort);
      SocketClose(socketHandle);
      return INIT_FAILED;
   }
   
   // Set trade parameters
   trade.SetExpertMagicNumber(MagicNumber);
   trade.SetDeviationInPoints(Slippage);
   trade.SetTypeFilling(ORDER_FILLING_FOK);
   
   Print("[RECEIVER] Socket bound successfully");
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
   // Check for incoming UDP packets
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
      
      // Parse and execute trade
      if(ParseAndExecuteTrade(data))
         Print("[RECEIVER] Trade executed successfully");
      else
         Print("[RECEIVER] Failed to execute trade");
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
   string cmd = "";
   
   // Extract fields from JSON
   if(!ExtractJSONField(json_data, "id", trade_id)) return false;
   if(!ExtractJSONField(json_data, "symbol", symbol)) return false;
   if(!ExtractJSONField(json_data, "type", trade_type)) return false;
   if(!ExtractJSONField(json_data, "lots", lots)) return false;
   
   ExtractJSONField(json_data, "price", price);
   ExtractJSONField(json_data, "sl", sl);
   ExtractJSONField(json_data, "tp", tp);
   ExtractJSONField(json_data, "cmd", cmd);
   
   Print("[RECEIVER] Parsed trade: ID=", trade_id, " Symbol=", symbol, " Type=", trade_type, " Lots=", lots);
   
   // Execute trade
   bool success = false;
   
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
      Print("[RECEIVER] Trade executed: ", trade.ResultOrder());
      
      // Send ACK
      SendAck(trade_id);
      return true;
   }
   else
   {
      Print("[RECEIVER] Trade failed: ", trade.ResultRetcodeDescription());
      return false;
   }
}

//+------------------------------------------------------------------+
//| Send ACK response                                                |
//+------------------------------------------------------------------+
void SendAck(ulong trade_id)
{
   string ack_json = "{\\\"ack\\\":" + IntegerToString(trade_id) + "}";
   
   // Convert to char array
   uchar data[];
   StringToCharArray(ack_json, data, 0, StringLen(ack_json));
   
   int sent = SocketSend(socketHandle, data, ArraySize(data));
   
   if(sent > 0)
      Print("[RECEIVER] ACK sent for trade ", trade_id, " (", sent, " bytes)");
   else
      Print("[RECEIVER] ERROR: Failed to send ACK, error: ", GetLastError());
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
