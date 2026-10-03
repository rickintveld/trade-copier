---
name: change-trade-wire-format
description: Checklist for changing the Trade JSON exchanged between the Signal Provider EA, the Rust router/workers and the Signal Receiver EA. Use whenever types.rs Trade, ProfitInfo, or the ack/PING line protocol changes.
---

# Change the master/slave wire format

`Trade` in `backend/src/types.rs` is the newline-delimited JSON contract with **both** MQL5 EAs:

```
Signal Provider EA --TCP :5000--> router.rs --broadcast--> worker.rs --TCP--> Signal Receiver EA
                                                           <-- "OK:<id>" / "ERROR:..." ack, {"profit":..} JSON
worker heartbeat: "PING\n"
```

## Rules

- Keep it backward compatible: users may run an older EA against a newer app. New fields must be `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]` or have `#[serde(default)]`.
- Serde names are the wire names (`trade_type` is `"type"` on the wire; `cmd` defaults to `"open"`).
- The worker mutates trades before forwarding (`adjust_trade_for_slave`: lots × multiplier rounded to 2 decimals, `symbol_prefix` appended as a suffix). Changes there are trading-behaviour changes — test them.

## Checklist

1. `backend/src/types.rs` — update the struct and its serde tests (round-trip, missing optional fields, old payloads still parse).
2. `backend/src/mql5/Trading Rocket/Signal Provider.mq5` — JSON it builds.
3. `backend/src/mql5/Trading Rocket/Signal Receiver.mq5` — JSON it parses / acks it sends.
4. **The `.ex5` files must be recompiled in MetaEditor by a human.** An agent cannot do this — say so explicitly in the hand-off.
5. `ea_sync.rs` copies the EAs to the master terminal; no change needed unless file names change.
6. Update `docs/mql5-signal-provider.md`, `docs/mql5-signal-receiver.md`, `docs/router.md`/`docs/workers.md` as relevant.
7. `cargo test` — includes the end-to-end worker test with a fake receiver in `worker.rs`.
