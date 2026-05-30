use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};

use crate::graph::CellGraph;

// Magic bytes: "GPAT"
const MAGIC: [u8; 4] = [b'G', b'P', b'A', b'T'];
const VERSION: [u8; 2] = [0, 1];

// ─── Binary format ───

pub fn save_binary(graph: &CellGraph, path: &str) -> io::Result<()> {
    let file = fs::File::create(path)?;
    let mut w = BufWriter::new(file);

    // Header
    w.write_all(&MAGIC)?;
    w.write_all(&VERSION)?;
    w.write_all(&(graph.rooms.len() as u32).to_le_bytes())?;
    w.write_all(&(graph.edges.len() as u32).to_le_bytes())?;

    // Rooms: 20 bytes each (id:4 + vibe:8 + surprise:8)
    for room in &graph.rooms {
        w.write_all(&(room.id as u32).to_le_bytes())?;
        w.write_all(&room.vibe.to_le_bytes())?;
        w.write_all(&room.surprise.to_le_bytes())?;
    }

    // Edges: 20 bytes each (from:4 + to:4 + weight:8)
    for edge in &graph.edges {
        w.write_all(&(edge.from as u32).to_le_bytes())?;
        w.write_all(&(edge.to as u32).to_le_bytes())?;
        w.write_all(&edge.weight.to_le_bytes())?;
    }

    w.flush()?;
    Ok(())
}

pub fn load_binary(path: &str) -> io::Result<CellGraph> {
    let file = fs::File::open(path)?;
    let mut r = BufReader::new(file);

    // Header
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid magic bytes"));
    }

    let mut version = [0u8; 2];
    r.read_exact(&mut version)?;
    if version[0] > VERSION[0] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported version: {}.{}", version[0], version[1]),
        ));
    }

    let mut buf4 = [0u8; 4];
    r.read_exact(&mut buf4)?;
    let room_count = u32::from_le_bytes(buf4) as usize;

    r.read_exact(&mut buf4)?;
    let edge_count = u32::from_le_bytes(buf4) as usize;

    let mut graph = CellGraph::new();

    for _ in 0..room_count {
        r.read_exact(&mut buf4)?;
        let id = u32::from_le_bytes(buf4) as usize;
        let mut buf8 = [0u8; 8];
        r.read_exact(&mut buf8)?;
        let vibe = f64::from_le_bytes(buf8);
        r.read_exact(&mut buf8)?;
        let surprise = f64::from_le_bytes(buf8);
        let idx = graph.add_room(vibe, surprise);
        debug_assert_eq!(idx, id);
    }

    for _ in 0..edge_count {
        r.read_exact(&mut buf4)?;
        let from = u32::from_le_bytes(buf4) as usize;
        r.read_exact(&mut buf4)?;
        let to = u32::from_le_bytes(buf4) as usize;
        let mut buf8 = [0u8; 8];
        r.read_exact(&mut buf8)?;
        let weight = f64::from_le_bytes(buf8);
        graph.add_edge(from, to, weight);
    }

    Ok(graph)
}

// ─── JSON format (manual, zero-dep) ───

pub fn save_json(graph: &CellGraph, path: &str) -> io::Result<()> {
    let mut s = String::from("{\"rooms\":[");
    for (i, room) in graph.rooms.iter().enumerate() {
        if i > 0 { s.push(','); }
        s.push_str(&format!(
            "{{\"id\":{},\"vibe\":{},\"surprise\":{}}}",
            room.id, room.vibe, room.surprise
        ));
    }
    s.push_str("],\"edges\":[");
    for (i, edge) in graph.edges.iter().enumerate() {
        if i > 0 { s.push(','); }
        s.push_str(&format!(
            "{{\"from\":{},\"to\":{},\"weight\":{}}}",
            edge.from, edge.to, edge.weight
        ));
    }
    s.push_str("]}");
    fs::write(path, s)
}

pub fn load_json(path: &str) -> io::Result<CellGraph> {
    let s = fs::read_to_string(path)?;
    parse_json_graph(&s)
}

fn parse_json_graph(s: &str) -> io::Result<CellGraph> {
    let mut graph = CellGraph::new();

    // Find rooms array
    let rooms_start = s.find("\"rooms\":[")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing rooms array"))?
        + "\"rooms\":[".len();

    let rooms_end = find_array_end(&s[rooms_start..])
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unclosed rooms array"))?;
    let rooms_str = &s[rooms_start..rooms_start + rooms_end];

    // Parse room objects
    for obj_str in split_objects(rooms_str) {
        let id = json_get_usize(obj_str, "id")?;
        let vibe = json_get_f64(obj_str, "vibe")?;
        let surprise = json_get_f64(obj_str, "surprise")?;
        let idx = graph.add_room(vibe, surprise);
        debug_assert_eq!(idx, id);
    }

    // Find edges array
    let edges_start = s.find("\"edges\":[")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing edges array"))?
        + "\"edges\":[".len();

    let rest = &s[edges_start..];
    let edges_end = find_array_end(rest)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unclosed edges array"))?;
    let edges_str = &rest[..edges_end];

    for obj_str in split_objects(edges_str) {
        let from = json_get_usize(obj_str, "from")?;
        let to = json_get_usize(obj_str, "to")?;
        let weight = json_get_f64(obj_str, "weight")?;
        graph.add_edge(from, to, weight);
    }

    Ok(graph)
}

fn find_array_end(s: &str) -> Option<usize> {
    // s is content AFTER the opening '['
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

fn split_objects(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = None;
    for (i, c) in s.char_indices() {
        match c {
            '{' => {
                if depth == 0 { start = Some(i); }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s_idx) = start {
                        result.push(&s[s_idx..=i]);
                    }
                }
            }
            _ => {}
        }
    }
    result
}

fn json_get_f64(obj: &str, key: &str) -> io::Result<f64> {
    let needle = format!("\"{}\":", key);
    let start = obj.find(&needle)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing key: {}", key)))?
        + needle.len();
    let rest = &obj[start..];
    let end = rest.find(|c: char| c == ',' || c == '}')
        .unwrap_or(rest.len());
    rest[..end].trim().parse::<f64>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, format!("invalid number for key: {}", key)))
}

fn json_get_usize(obj: &str, key: &str) -> io::Result<usize> {
    let v = json_get_f64(obj, key)?;
    Ok(v as usize)
}

// ─── CSV format ───

pub fn export_rooms_csv(graph: &CellGraph, path: &str) -> io::Result<()> {
    let mut s = String::from("id,vibe,surprise\n");
    for room in &graph.rooms {
        s.push_str(&format!("{},{},{}\n", room.id, room.vibe, room.surprise));
    }
    fs::write(path, s)
}

pub fn export_edges_csv(graph: &CellGraph, path: &str) -> io::Result<()> {
    let mut s = String::from("from,to,weight\n");
    for edge in &graph.edges {
        s.push_str(&format!("{},{},{}\n", edge.from, edge.to, edge.weight));
    }
    fs::write(path, s)
}

pub fn export_tick_history_csv(history: &[(u64, f64, f64)], path: &str) -> io::Result<()> {
    let mut s = String::from("tick,mean_vibe,mean_surprise\n");
    for (tick, vibe, surprise) in history {
        s.push_str(&format!("{},{},{}\n", tick, vibe, surprise));
    }
    fs::write(path, s)
}
