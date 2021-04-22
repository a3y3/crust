use gotham_derive::StateData;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, UdpSocket};
use std::sync::{Arc, Mutex};

use std::env;

const M: u64 = 64;

enum Bracket {
    Open,
    Closed,
}

struct Interval {
    bracket1: Bracket,
    val1: u64,
    bracket2: Bracket,
    val2: u64,
}

impl Interval {
    fn new(bracket1: Bracket, val1: u64, bracket2: Bracket, val2: u64) -> Self {
        Interval {
            bracket1,
            val1,
            bracket2,
            val2,
        }
    }

    /// Needs improvement - right now this method uses a pretty inefficient way to check if a `val` lies between an `Interval. Selecting a `start` and and `end` and walking through each value between them isn't ideal, and we need  a faster way to do this.
    fn is_in_interval(&self, val: u64) -> bool {
        let mut start = match self.bracket1 {
            Bracket::Open => (self.val1 + 1) % M,
            Bracket::Closed => self.val1,
        };
        let end = match self.bracket2 {
            Bracket::Open => (self.val2 - 1) % M,
            Bracket::Closed => self.val2,
        };

        while start != end {
            if start == val {
                return true;
            }
            start += 1;
        }
        val == start
    }
}

struct FingerTableEntry {
    start: u64,
    interval: Interval,
    successor: u64,
    node_ip: IpAddr,
}

impl FingerTableEntry {
    fn new(start: u64, interval: Interval, successor: u64, node_ip: IpAddr) -> Self {
        FingerTableEntry {
            start,
            interval,
            successor,
            node_ip,
        }
    }
}

struct FingerTable {
    finger_table: Vec<FingerTableEntry>,
}

impl FingerTable {
    fn new() -> Self {
        FingerTable {
            finger_table: Vec::new(),
        }
    }

    fn add_entry(&mut self, entry: FingerTableEntry) {
        self.finger_table.push(entry);
    }
}

#[derive(Clone, StateData)]
pub struct Chord {
    finger_table: Arc<Mutex<FingerTable>>,
    hash_map: Arc<Mutex<HashMap<String, String>>>,
}

impl Chord {
    fn new(finger_table: FingerTable, hash_map: HashMap<String, String>) -> Self {
        let finger_table = Arc::new(Mutex::new(finger_table));
        let hash_map = Arc::new(Mutex::new(hash_map));
        Chord {
            finger_table,
            hash_map,
        }
    }

    pub fn get_successor(&self) -> IpAddr {
        let table = self.finger_table.lock().unwrap();
        (*table).finger_table.get(0).unwrap().node_ip
    }
}

pub fn initialize_node() -> Chord {
    let args: Vec<String> = env::args().collect();
    let mut finger_table = FingerTable::new();
    let hash_map = HashMap::new();
    if args.len() == 1 {
        // first node
        let self_ip = get_self_ip();
        let self_id = get_identifier(self_ip.to_string());
        let start = (self_id + 1) % M;
        let interval = Interval::new(Bracket::Closed, start, Bracket::Closed, start);
        let first_entry = FingerTableEntry::new(start, interval, self_id, self_ip);
        finger_table.add_entry(first_entry);
    } else {
        // contact the node to:
        // 1. update its successor
        // 2. transfer its keys to this node
        unimplemented!()
    }

    Chord::new(finger_table, hash_map)
}

fn get_self_ip() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("8.8.8.8:80").unwrap();
    socket.local_addr().unwrap().ip()
}

fn get_identifier(key: String) -> u64 {
    hasher(key) % M
}

fn hasher(key: String) -> u64 {
    let mut s = DefaultHasher::new();
    key.hash(&mut s);
    s.finish()
}
