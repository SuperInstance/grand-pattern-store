# grand-pattern-store

Persistence layer for the Grand Pattern cell graph. Save and restore graph state.

## Features

- **Binary format** — compact, fast serialization
- **JSON format** — human-readable, interoperable (zero external dependencies)
- **CSV format** — analysis-friendly exports (rooms, edges, tick history)
- **Append-only tick log** — for replay and audit
- **Snapshot + restore** — with JEPA state preservation

## Usage

```rust
use grand_pattern_store::graph::CellGraph;
use grand_pattern_store::persistence::*;
use grand_pattern_store::tick_log::TickLog;
use grand_pattern_store::snapshot::Snapshot;

let mut graph = CellGraph::new();
let r0 = graph.add_room(0.5, 0.1);
let r1 = graph.add_room(0.8, 0.3);
graph.add_edge(r0, r1, 0.7);

// Binary
save_binary(&graph, "graph.bin")?;
let loaded = load_binary("graph.bin")?;

// JSON
save_json(&graph, "graph.json")?;
let loaded = load_json("graph.json")?;

// CSV
export_rooms_csv(&graph, "rooms.csv")?;
export_edges_csv(&graph, "edges.csv")?;

// Tick log
let mut log = TickLog::create("tick.log")?;
log.append(1, &[0.5, 0.8], &[0.1, 0.3])?;
let entries = TickLog::replay("tick.log")?;

// Snapshot
let snap = Snapshot::from_graph(42, &graph, &readings, &weights);
snap.save("snapshot.json")?;
let loaded = Snapshot::load("snapshot.json")?;
```

## Binary Format

```
[magic:4 "GPAT"][version:2][room_count:4][edge_count:4]
[rooms: room_count * 20 bytes (id:4 + vibe:8 + surprise:8)]
[edges: edge_count * 20 bytes (from:4 + to:4 + weight:8)]
```

## License

MIT
