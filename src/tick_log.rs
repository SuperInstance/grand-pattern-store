use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};

/// Append-only tick log for replay.
pub struct TickLog {
    file: File,
}

impl TickLog {
    pub fn create(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(TickLog { file })
    }

    pub fn append(&mut self, tick: u64, rooms: &[f64], surprises: &[f64]) -> io::Result<()> {
        // Format: tick room_count room0 room1 ... surprise0 surprise1 ...
        write!(self.file, "{} {}", tick, rooms.len())?;
        for v in rooms {
            write!(self.file, " {}", v)?;
        }
        for v in surprises {
            write!(self.file, " {}", v)?;
        }
        writeln!(self.file)?;
        self.file.flush()?;
        Ok(())
    }

    pub fn replay(path: &str) -> io::Result<Vec<(u64, Vec<f64>, Vec<f64>)>> {
        let file = File::open(path)?;
        let mut result = Vec::new();
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }
            let tick: u64 = parts[0].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad tick"))?;
            let count: usize = parts[1].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad count"))?;
            if parts.len() < 2 + count * 2 { continue; }
            let rooms: Vec<f64> = parts[2..2 + count]
                .iter()
                .map(|s| s.parse::<f64>().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad room val")))
                .collect::<Result<_, _>>()?;
            let surprises: Vec<f64> = parts[2 + count..2 + count * 2]
                .iter()
                .map(|s| s.parse::<f64>().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad surprise val")))
                .collect::<Result<_, _>>()?;
            result.push((tick, rooms, surprises));
        }
        Ok(result)
    }

    /// Check if file is opened in append mode (always true for TickLog).
    pub fn is_append_only(&self) -> bool {
        true
    }
}
