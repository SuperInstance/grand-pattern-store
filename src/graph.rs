/// Core data structures for the Grand Pattern cell graph.

/// A room (node) in the cell graph.
#[derive(Debug, Clone)]
pub struct Room {
    pub id: usize,
    pub vibe: f64,
    pub surprise: f64,
}

/// The cell graph: a collection of rooms with weighted edges.
#[derive(Debug, Clone)]
pub struct CellGraph {
    pub rooms: Vec<Room>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

impl CellGraph {
    pub fn new() -> Self {
        CellGraph {
            rooms: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_room(&mut self, vibe: f64, surprise: f64) -> usize {
        let id = self.rooms.len();
        self.rooms.push(Room { id, vibe, surprise });
        id
    }

    pub fn add_edge(&mut self, from: usize, to: usize, weight: f64) {
        self.edges.push(Edge { from, to, weight });
    }
}

impl Default for CellGraph {
    fn default() -> Self {
        Self::new()
    }
}
