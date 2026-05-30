use std::fs;
use std::io;
use std::time::Instant;

use grand_pattern_store::graph::CellGraph;
use grand_pattern_store::persistence::*;
use grand_pattern_store::tick_log::TickLog;
use grand_pattern_store::snapshot::{Snapshot, RoomSnapshot};

fn make_graph() -> CellGraph {
    let mut g = CellGraph::new();
    let r0 = g.add_room(0.5, 0.1);
    let r1 = g.add_room(0.8, 0.3);
    let r2 = g.add_room(0.2, 0.9);
    g.add_edge(r0, r1, 0.7);
    g.add_edge(r1, r2, 0.4);
    g
}

#[test]
fn test_binary_roundtrip() {
    let g = make_graph();
    let path = "/tmp/gp_test_binary.bin";
    save_binary(&g, path).unwrap();
    let loaded = load_binary(path).unwrap();
    assert_eq!(loaded.rooms.len(), 3);
    assert_eq!(loaded.edges.len(), 2);
    assert!((loaded.rooms[0].vibe - 0.5).abs() < 1e-10);
    assert!((loaded.rooms[2].surprise - 0.9).abs() < 1e-10);
    assert!((loaded.edges[0].weight - 0.7).abs() < 1e-10);
}

#[test]
fn test_json_roundtrip() {
    let g = make_graph();
    let path = "/tmp/gp_test_json.json";
    save_json(&g, path).unwrap();
    let loaded = load_json(path).unwrap();
    assert_eq!(loaded.rooms.len(), 3);
    assert_eq!(loaded.edges.len(), 2);
    assert!((loaded.rooms[1].vibe - 0.8).abs() < 1e-10);
    assert!((loaded.edges[1].weight - 0.4).abs() < 1e-10);
}

#[test]
fn test_csv_rooms() {
    let g = make_graph();
    let path = "/tmp/gp_test_rooms.csv";
    export_rooms_csv(&g, path).unwrap();
    let content = fs::read_to_string(path).unwrap();
    assert!(content.starts_with("id,vibe,surprise\n"));
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 4); // header + 3 rooms
}

#[test]
fn test_csv_edges() {
    let g = make_graph();
    let path = "/tmp/gp_test_edges.csv";
    export_edges_csv(&g, path).unwrap();
    let content = fs::read_to_string(path).unwrap();
    assert!(content.starts_with("from,to,weight\n"));
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3); // header + 2 edges
}

#[test]
fn test_csv_tick_history() {
    let history = vec![(1u64, 0.5f64, 0.1f64), (2u64, 0.6, 0.2)];
    let path = "/tmp/gp_test_tick_history.csv";
    export_tick_history_csv(&history, path).unwrap();
    let content = fs::read_to_string(path).unwrap();
    assert!(content.starts_with("tick,mean_vibe,mean_surprise\n"));
}

#[test]
fn test_tick_log_append_replay() {
    let path = "/tmp/gp_test_ticklog.log";
    let _ = fs::remove_file(path);
    let mut log = TickLog::create(path).unwrap();
    log.append(1, &[0.5, 0.8], &[0.1, 0.3]).unwrap();
    log.append(2, &[0.6, 0.9], &[0.2, 0.4]).unwrap();

    let entries = TickLog::replay(path).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, 1);
    assert!((entries[0].1[0] - 0.5).abs() < 1e-10);
    assert!((entries[1].2[1] - 0.4).abs() < 1e-10);
}

#[test]
fn test_snapshot_preserves_vibe() {
    let g = make_graph();
    let jepa_readings = vec![
        vec![(1.0, 0.5), (2.0, 0.6)],
        vec![(1.0, 0.8)],
        vec![],
    ];
    let jepa_weights = vec![
        vec![0.1, 0.2],
        vec![0.3],
        vec![0.4, 0.5, 0.6],
    ];
    let snap = Snapshot::from_graph(42, &g, &jepa_readings, &jepa_weights);
    let restored = snap.to_graph();
    assert!((restored.rooms[0].vibe - 0.5).abs() < 1e-10);
    assert!((restored.rooms[1].surprise - 0.3).abs() < 1e-10);
}

#[test]
fn test_snapshot_preserves_edges() {
    let g = make_graph();
    let snap = Snapshot::from_graph(0, &g, &[], &[]);
    assert_eq!(snap.edges.len(), 2);
    let restored = snap.to_graph();
    assert_eq!(restored.edges.len(), 2);
    assert_eq!(restored.edges[0].from, 0);
    assert_eq!(restored.edges[0].to, 1);
}

#[test]
fn test_snapshot_preserves_jepa_weights() {
    let g = make_graph();
    let jepa_weights = vec![vec![0.1, 0.2], vec![0.3], vec![]];
    let snap = Snapshot::from_graph(0, &g, &[], &jepa_weights);
    assert_eq!(snap.rooms[0].jepa_weights.len(), 2);
    assert!((snap.rooms[0].jepa_weights[0] - 0.1).abs() < 1e-10);
    assert_eq!(snap.rooms[2].jepa_weights.len(), 0);
}

#[test]
fn test_binary_format_compact() {
    let g = make_graph();
    let path = "/tmp/gp_test_compact.bin";
    save_binary(&g, path).unwrap();
    let size = fs::metadata(path).unwrap().len();
    // Header: 14 bytes, rooms: 3*20=60, edges: 2*16=32 → 106
    assert_eq!(size, 106);
}

#[test]
fn test_json_is_valid() {
    let g = make_graph();
    let path = "/tmp/gp_test_valid.json";
    save_json(&g, path).unwrap();
    let content = fs::read_to_string(path).unwrap();
    // Check it starts and ends correctly
    assert!(content.starts_with('{'));
    assert!(content.ends_with('}'));
    // Try to parse it back (our own parser is the validity check)
    let loaded = load_json(path).unwrap();
    assert_eq!(loaded.rooms.len(), 3);
}

#[test]
fn test_csv_spreadsheet_friendly() {
    let g = make_graph();
    let path = "/tmp/gp_test_spread.csv";
    export_rooms_csv(&g, path).unwrap();
    let content = fs::read_to_string(path).unwrap();
    // No commas in numbers (f64 display)
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(parts.len(), 3);
    }
}

#[test]
fn test_tick_log_10k_ticks() {
    let path = "/tmp/gp_test_10k.log";
    let _ = fs::remove_file(path);
    let mut log = TickLog::create(path).unwrap();
    for tick in 0..10_000u64 {
        log.append(tick, &[tick as f64 * 0.01], &[tick as f64 * 0.02]).unwrap();
    }
    let entries = TickLog::replay(path).unwrap();
    assert_eq!(entries.len(), 10_000);
}

#[test]
fn test_load_nonexistent_file() {
    let result = load_binary("/tmp/gp_test_does_not_exist_12345.bin");
    assert!(result.is_err());
    let result = load_json("/tmp/gp_test_does_not_exist_12345.json");
    assert!(result.is_err());
}

#[test]
fn test_save_readonly_dir() {
    let result = save_binary(&CellGraph::new(), "/proc/gp_test.bin");
    assert!(result.is_err());
}

#[test]
fn test_empty_graph_roundtrip() {
    let g = CellGraph::new();
    let path = "/tmp/gp_test_empty.bin";
    save_binary(&g, path).unwrap();
    let loaded = load_binary(path).unwrap();
    assert_eq!(loaded.rooms.len(), 0);
    assert_eq!(loaded.edges.len(), 0);
}

#[test]
fn test_empty_graph_json_roundtrip() {
    let g = CellGraph::new();
    let path = "/tmp/gp_test_empty.json";
    save_json(&g, path).unwrap();
    let loaded = load_json(path).unwrap();
    assert_eq!(loaded.rooms.len(), 0);
    assert_eq!(loaded.edges.len(), 0);
}

#[test]
fn test_single_room_save_load() {
    let mut g = CellGraph::new();
    g.add_room(42.0, 99.0);
    let path = "/tmp/gp_test_single.bin";
    save_binary(&g, path).unwrap();
    let loaded = load_binary(path).unwrap();
    assert_eq!(loaded.rooms.len(), 1);
    assert!((loaded.rooms[0].vibe - 42.0).abs() < 1e-10);
}

#[test]
fn test_large_graph_save_load() {
    let mut g = CellGraph::new();
    for i in 0..1000 {
        g.add_room(i as f64, (1000 - i) as f64);
    }
    for i in 0..999 {
        g.add_edge(i, i + 1, 1.0 / (i as f64 + 1.0));
    }
    let path = "/tmp/gp_test_large.bin";
    save_binary(&g, path).unwrap();
    let loaded = load_binary(path).unwrap();
    assert_eq!(loaded.rooms.len(), 1000);
    assert_eq!(loaded.edges.len(), 999);
}

#[test]
fn test_binary_version_check() {
    // Create a file with future version
    let data = b"GPAT\x00\x02\x00\x00\x00\x00\x00\x00\x00";
    fs::write("/tmp/gp_test_version.bin", data).unwrap();
    let result = load_binary("/tmp/gp_test_version.bin");
    // Should succeed since major version is 0 and our version is also 0
    // Actually 0.2 > 0.1, but major is 0 so it passes. Let me check logic.
    // The check is version[0] > VERSION[0]. Both are 0, so it passes.
    // Let me test with major version 1
    let data2 = b"GPAT\x01\x00\x00\x00\x00\x00\x00\x00\x00";
    fs::write("/tmp/gp_test_version2.bin", data2).unwrap();
    let result2 = load_binary("/tmp/gp_test_version2.bin");
    assert!(result2.is_err());
}

#[test]
fn test_corrupted_binary_detected() {
    // Bad magic
    let data = b"XXXX\x00\x01\x00\x00\x00\x00\x00\x00\x00";
    fs::write("/tmp/gp_test_corrupt.bin", data).unwrap();
    let result = load_binary("/tmp/gp_test_corrupt.bin");
    assert!(result.is_err());
}

#[test]
fn test_tick_log_append_only() {
    let path = "/tmp/gp_test_append.log";
    let _ = fs::remove_file(path);
    let mut log = TickLog::create(path).unwrap();
    log.append(1, &[0.5], &[0.1]).unwrap();
    assert!(log.is_append_only());
    drop(log);

    // Reopen and append more
    let mut log2 = TickLog::create(path).unwrap();
    log2.append(2, &[0.6], &[0.2]).unwrap();
    drop(log2);

    let entries = TickLog::replay(path).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_multiple_snapshots() {
    let g = make_graph();
    let snap1 = Snapshot::from_graph(1, &g, &[], &[]);
    let snap2 = Snapshot::from_graph(2, &g, &[], &[]);

    snap1.save("/tmp/gp_snap1.json").unwrap();
    snap2.save("/tmp/gp_snap2.json").unwrap();

    let loaded1 = Snapshot::load("/tmp/gp_snap1.json").unwrap();
    let loaded2 = Snapshot::load("/tmp/gp_snap2.json").unwrap();

    assert_eq!(loaded1.tick, 1);
    assert_eq!(loaded2.tick, 2);
    assert_eq!(loaded1.rooms.len(), loaded2.rooms.len());
}

#[test]
fn test_snapshot_with_jepa_roundtrip() {
    let g = make_graph();
    let readings = vec![
        vec![(1.0, 0.5), (2.0, 0.6)],
        vec![(1.0, 0.8)],
        vec![(3.0, 0.1), (4.0, 0.2)],
    ];
    let weights = vec![
        vec![0.1, 0.2, 0.3],
        vec![0.4],
        vec![0.5, 0.6],
    ];
    let snap = Snapshot::from_graph(99, &g, &readings, &weights);
    snap.save("/tmp/gp_snap_jepa.json").unwrap();
    let loaded = Snapshot::load("/tmp/gp_snap_jepa.json").unwrap();

    assert_eq!(loaded.tick, 99);
    assert_eq!(loaded.rooms[0].jepa_readings.len(), 2);
    assert!((loaded.rooms[0].jepa_readings[0].0 - 1.0).abs() < 1e-10);
    assert!((loaded.rooms[0].jepa_readings[1].1 - 0.6).abs() < 1e-10);
    assert_eq!(loaded.rooms[1].jepa_readings.len(), 1);
    assert_eq!(loaded.rooms[2].jepa_readings.len(), 2);
    assert_eq!(loaded.rooms[0].jepa_weights.len(), 3);
    assert!((loaded.rooms[0].jepa_weights[2] - 0.3).abs() < 1e-10);
}

#[test]
fn test_performance_10k_rooms() {
    let mut g = CellGraph::new();
    for i in 0..10_000 {
        g.add_room(i as f64 * 0.001, (10000 - i) as f64 * 0.001);
    }
    let path = "/tmp/gp_test_perf.bin";
    let start = Instant::now();
    save_binary(&g, path).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 100, "save took {:?}", elapsed);

    let start = Instant::now();
    let loaded = load_binary(path).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 100, "load took {:?}", elapsed);
    assert_eq!(loaded.rooms.len(), 10_000);
}

#[test]
fn test_snapshot_restore_from_graph() {
    let g = make_graph();
    let snap = Snapshot::from_graph(10, &g, &[], &[]);
    let restored = snap.to_graph();
    assert_eq!(restored.rooms.len(), g.rooms.len());
    assert_eq!(restored.edges.len(), g.edges.len());
    for (a, b) in restored.rooms.iter().zip(g.rooms.iter()) {
        assert!((a.vibe - b.vibe).abs() < 1e-10);
        assert!((a.surprise - b.surprise).abs() < 1e-10);
    }
}
