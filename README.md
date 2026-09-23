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

## Roadmap

### `bjle-shared` — Tipi condivisi e serializzazione
> Definisce i messaggi che circolano sulla mesh. Dipendenza zero degli altri crate.

- [X] Struct `Card { suit: Suit, rank: Rank }` + enum `Suit`, `Rank`
- [ ] Calcolo valore mano (asso 1 o 11, logica soft hand)
- [X] Enum `GameEvent` — tutti i messaggi di gioco:
  - `Join { player_id, room_id }`
  - `Deal { seed: u64 }` (seed condiviso per mazzo deterministico)
  - `PlayerAction { player_id, action: Action }` (`Hit` / `Stand`)
  - `Leave { player_id }`
- [X] Struct `MeshPacket { msg_id: u128, room_id: String, ttl: u8, payload: GameEvent }`
- [ ] Serializzazione `serde` + `postcard` (no_std-friendly, compatto su BLE)
- [ ] Test unitari encode/decode round-trip

---

### `bjle-engine` — Motore di gioco (FSM)
> Logica pura, nessuna I/O. Riceve eventi, emette eventi validati.

- [ ] Enum `GameState`: `Lobby → Dealing → PlayerTurns → DealerTurn → Payout → Lobby`
- [ ] Struct `TableState { deck_seed, hands: HashMap<PlayerId, Hand>, dealer_hand, current_player, phase }`
- [ ] Generazione mazzo deterministico con `ChaCha8Rng` da seed condiviso
- [ ] Distribuzione iniziale (2 carte a ogni giocatore + dealer, carta coperta dealer)
- [ ] Validazione mosse: `can_act(player_id) -> bool`
- [ ] Logica `Hit`: pesca carta, controlla bust (> 21)
- [ ] Logica `Stand`: passa al prossimo giocatore / fase dealer
- [ ] Logica banco virtuale: dealer pesca finché `hand_value < 17`
- [ ] Calcolo payout (BlackJack naturale 3:2, win 1:1, bust/dealer-win 0)
- [ ] API `fn apply(state: &mut TableState, event: GameEvent) -> Result<Vec<GameEvent>>`
- [ ] Test FSM: sequenza completa da `Deal` a `Payout`

---

### `bjle-mesh` — Livello mesh (Gossip / Relay)
> Trasporto BLE. Riceve byte da `bluer`, consegna `GameEvent` all'engine, ritrasmette con TTL.

- [ ] Inizializzazione adapter BLE con `bluer` (controlla permessi `bluetoothd`)
- [ ] **Broadcaster**: advertising `ManufacturerData` con payload `MeshPacket` serializzato
- [ ] **Scanner**: scanning continuo, parsing `ManufacturerData` → `MeshPacket`
- [ ] Filtro `room_id`: scarta pacchetti di altre stanze
- [ ] Deduplicazione: `HashSet<u128>` degli `msg_id` già visti (sliding window o LRU)
- [ ] Relay gossip: se `ttl > 0`, decrementa e re-advertise (rimbalzo tra nodi)
- [ ] Channel `tokio::sync::mpsc` verso engine (rx eventi in ingresso) e verso TUI (tx stato)
- [ ] Gestione errori BLE: adapter non trovato, permessi negati, timeout scan
- [ ] Test integrazione: due istanze locali su loopback mock

---

### `bjle-tui` — Interfaccia utente (Ratatui)
> Presenta lo stato del tavolo, raccoglie input, invia azioni all'engine via mesh.

- [ ] Setup terminale raw mode + `CrosstermBackend`
- [ ] Layout: pannello mano giocatore | mano dealer | log eventi | stato stanza
- [ ] Rendering carte ASCII (`[A♠]`, `[K♥]`, `[?]` per carta coperta)
- [ ] Schermata lobby: inserimento `room_id` e `player_id`
- [ ] Input: `h` = Hit, `s` = Stand, `q` = quit
- [ ] Highlight turno attivo (bordo colorato sul pannello del giocatore corrente)
- [ ] Pannello log: ultime N azioni ricevute dalla mesh
- [ ] Schermata payout: mostra risultati e opzione "nuova mano"
- [ ] Gestione resize terminale
- [ ] Tick loop `tokio` asincrono: aggiorna UI ogni 100ms o su evento mesh
