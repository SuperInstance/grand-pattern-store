use std::fs;
use std::io;

use crate::graph::CellGraph;

#[derive(Debug, Clone)]
pub struct RoomSnapshot {
    pub id: usize,
    pub vibe: f64,
    pub surprise: f64,
    pub jepa_readings: Vec<(f64, f64)>,
    pub jepa_weights: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub tick: u64,
    pub rooms: Vec<RoomSnapshot>,
    pub edges: Vec<(usize, usize, f64)>,
}

impl Snapshot {
    pub fn from_graph(tick: u64, graph: &CellGraph, jepa_readings: &[Vec<(f64, f64)>], jepa_weights: &[Vec<f64>]) -> Self {
        let rooms: Vec<RoomSnapshot> = graph.rooms.iter().enumerate().map(|(i, room)| {
            RoomSnapshot {
                id: room.id,
                vibe: room.vibe,
                surprise: room.surprise,
                jepa_readings: jepa_readings.get(i).cloned().unwrap_or_default(),
                jepa_weights: jepa_weights.get(i).cloned().unwrap_or_default(),
            }
        }).collect();
        let edges: Vec<(usize, usize, f64)> = graph.edges.iter()
            .map(|e| (e.from, e.to, e.weight))
            .collect();
        Snapshot { tick, rooms, edges }
    }

    pub fn to_graph(&self) -> CellGraph {
        let mut graph = CellGraph::new();
        for room in &self.rooms {
            let id = graph.add_room(room.vibe, room.surprise);
            debug_assert_eq!(id, room.id);
        }
        for &(from, to, weight) in &self.edges {
            graph.add_edge(from, to, weight);
        }
        graph
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        // Manual JSON snapshot format
        let mut s = String::from("{\"tick\":");
        s.push_str(&self.tick.to_string());
        s.push_str(",\"rooms\":[");
        for (i, room) in self.rooms.iter().enumerate() {
            if i > 0 { s.push(','); }
            s.push('{');
            s.push_str(&format!("\"id\":{},\"vibe\":{},\"surprise\":{},", room.id, room.vibe, room.surprise));
            s.push_str("\"jepa_readings\":[");
            for (j, &(ts, val)) in room.jepa_readings.iter().enumerate() {
                if j > 0 { s.push(','); }
                s.push_str(&format!("[{},{}]", ts, val));
            }
            s.push_str("],\"jepa_weights\":[");
            for (j, w) in room.jepa_weights.iter().enumerate() {
                if j > 0 { s.push(','); }
                s.push_str(&w.to_string());
            }
            s.push_str("]}");
        }
        s.push_str("],\"edges\":[");
        for (i, &(from, to, weight)) in self.edges.iter().enumerate() {
            if i > 0 { s.push(','); }
            s.push_str(&format!("[{},{},{}]", from, to, weight));
        }
        s.push_str("]}");
        fs::write(path, s)
    }

    pub fn load(path: &str) -> io::Result<Self> {
        let s = fs::read_to_string(path)?;
        parse_snapshot(&s)
    }
}

fn parse_snapshot(s: &str) -> io::Result<Snapshot> {
    // tick
    let tick_start = s.find("\"tick\":")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing tick"))?
        + "\"tick\":".len();
    let tick_rest = &s[tick_start..];
    let tick_end = tick_rest.find(',').unwrap_or(tick_rest.len());
    let tick: u64 = tick_rest[..tick_end].trim().parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad tick"))?;

    // rooms
    let rooms_key = "\"rooms\":[";
    let rooms_start = s.find(rooms_key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing rooms"))?
        + rooms_key.len();
    let rest = &s[rooms_start..];
    let rooms_end = find_array_end(rest)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unclosed rooms"))?;
    let rooms_str = &rest[..rooms_end];

    let mut rooms = Vec::new();
    // Simple object-by-object parsing
    let mut pos = 0;
    while pos < rooms_str.len() {
        if &rooms_str[pos..pos+1] == "{" {
            let obj_end = find_obj_end(&rooms_str[pos..])
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unclosed room obj"))?;
            let obj_str = &rooms_str[pos..pos + obj_end + 1];
            rooms.push(parse_room_snapshot(obj_str)?);
            pos += obj_end + 1;
        } else {
            pos += 1;
        }
    }

    // edges
    let edges_key = "\"edges\":[";
    let edges_start = s.find(edges_key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing edges"))?
        + edges_key.len();
    let rest = &s[edges_start..];
    let edges_end = find_array_end(rest)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unclosed edges"))?;
    let edges_str = &rest[..edges_end];

    let mut edges = Vec::new();
    let mut depth = 0;
    let mut arr_start = None;
    for (i, c) in edges_str.char_indices() {
        match c {
            '[' => { if depth == 0 { arr_start = Some(i); } depth += 1; }
            ']' => { depth -= 1; if depth == 0 {
                let a_s = arr_start.unwrap();
                let inner = &edges_str[a_s + 1..i];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() == 3 {
                    let from: usize = parts[0].trim().parse().unwrap();
                    let to: usize = parts[1].trim().parse().unwrap();
                    let weight: f64 = parts[2].trim().parse().unwrap();
                    edges.push((from, to, weight));
                }
            }}
            _ => {}
        }
    }

    Ok(Snapshot { tick, rooms, edges })
}

fn parse_room_snapshot(obj: &str) -> io::Result<RoomSnapshot> {
    let id: usize = get_json_num(obj, "id")? as usize;
    let vibe = get_json_num(obj, "vibe")?;
    let surprise = get_json_num(obj, "surprise")?;

    // jepa_weights
    let weights_key = "\"jepa_weights\":[";
    let mut jepa_weights = Vec::new();
    if let Some(idx) = obj.find(weights_key) {
        let w_start = idx + weights_key.len();
        let rest = &obj[w_start..];
        if let Some(end) = rest.find(']') {
            for part in rest[..end].split(',') {
                let v: f64 = part.trim().parse().unwrap_or(0.0);
                jepa_weights.push(v);
            }
        }
    }

    // jepa_readings
    let readings_key = "\"jepa_readings\":[";
    let mut jepa_readings = Vec::new();
    if let Some(idx) = obj.find(readings_key) {
        let r_start = idx + readings_key.len();
        let rest = &obj[r_start..];
        let r_end = find_array_end(rest).unwrap_or(0);
        let readings_str = &rest[..r_end];
        // Parse [ts,val] pairs
        let mut depth2 = 0;
        let mut inner_start = None;
        for (i, c) in readings_str.char_indices() {
            match c {
                '[' => { if depth2 == 0 { inner_start = Some(i); } depth2 += 1; }
                ']' => { depth2 -= 1; if depth2 == 0 {
                    if let Some(is) = inner_start {
                        let inner = &readings_str[is + 1..i];
                        let parts: Vec<&str> = inner.split(',').collect();
                        if parts.len() == 2 {
                            let ts: f64 = parts[0].trim().parse().unwrap_or(0.0);
                            let val: f64 = parts[1].trim().parse().unwrap_or(0.0);
                            jepa_readings.push((ts, val));
                        }
                    }
                }}
                _ => {}
            }
        }
    }

    Ok(RoomSnapshot { id, vibe, surprise, jepa_readings, jepa_weights })
}

fn get_json_num(obj: &str, key: &str) -> io::Result<f64> {
    let needle = format!("\"{}\":", key);
    let start = obj.find(&needle)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing key: {}", key)))?
        + needle.len();
    let rest = &obj[start..];
    let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
    rest[..end].trim().parse::<f64>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, format!("bad number: {}", key)))
}

fn find_array_end(s: &str) -> Option<usize> {
    // s is content AFTER the opening '[', so start at depth 1
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => { depth -= 1; if depth == 0 { return Some(i); } }
            _ => {}
        }
    }
    None
}

fn find_obj_end(s: &str) -> Option<usize> {
    // s starts with '{'
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => { depth -= 1; if depth == 0 { return Some(i); } }
            _ => {}
        }
    }
    None
}
