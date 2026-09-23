# BJ-LE
BlackJack su rete mesh

## Architettura
```
┌────────────────────────────────────────────────────────┐
│                   Interfaccia Utente                   │
│          (TUI con Ratatui / CLI da terminale)          │
└──────────────┬────────────────────────▲────────────────┘
               │ (Scelta: Hit/Stand)    │ (Stato del tavolo)
               ▼                        │
┌────────────────────────────────────────────────────────┐
│               Motore del Gioco (FSM)                   │
│   - Gestione mazzo deterministico (ChaCha8Rng)         │
│   - Banco Virtuale (calcola carte del dealer)          │
│   - Validatore mosse (è il turno di questo player?)    │
└──────────────┬────────────────────────▲────────────────┘
               │ Eventi validati        │ Eventi validati
               ▼                        │
┌────────────────────────────────────────────────────────┐
│             Livello Mesh (Gossip / Relay)              │
│   - Filtro Room ID ("RM01")                            │
│   - Deduplicazione messaggi (HashSet<msg_id>)          │
│   - Gestione TTL (Time-To-Live per il rimbalzo)        │
└──────────────┬────────────────────────▲────────────────┘
               │ Byte raw               │ Byte raw
               ▼                        │
┌────────────────────────────────────────────────────────┐
│             Driver Bluetooth LE (bluer)                │
│   - Broadcaster (Advertising via Manufacturer Data)    │
│   - Listener (Scanning continuo dei vicini)            │
└────────────────────────────────────────────────────────┘
```
