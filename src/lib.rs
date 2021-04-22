use gotham::handler::HandlerError;
use gotham_derive::StateData;
use reqwest::IntoUrl;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::env;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, UdpSocket};
use std::sync::{Arc, Mutex};

const M: u64 = 64;
const PORT: usize = 8000;

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
    fn new(bracket1: Bracket, val1: u64, val2: u64, bracket2: Bracket) -> Self {
        Interval {
            bracket1,
            val1,
            bracket2,
            val2,
        }
    }

    /// Needs improvement - right now this method uses a pretty inefficient way to check if a `val` lies between an `Interval. Selecting a `start` and and `end` and walking through each value between them isn't ideal, and we need  a faster way to do this.
    fn contains(&self, val: u64) -> bool {
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
pub struct ChordNode {
    finger_table: Arc<Mutex<FingerTable>>,
    hash_map: Arc<Mutex<HashMap<String, String>>>,
    self_ip: IpAddr,
}

impl ChordNode {
    fn new(finger_table: FingerTable, hash_map: HashMap<String, String>, self_ip: IpAddr) -> Self {
        let finger_table = Arc::new(Mutex::new(finger_table));
        let hash_map = Arc::new(Mutex::new(hash_map));
        ChordNode {
            finger_table,
            hash_map,
            self_ip,
        }
    }

    pub fn get_successor(&self) -> IpAddr {
        let table = self.finger_table.lock().unwrap();
        (*table).finger_table.get(0).unwrap().node_ip
    }

    pub async fn calculate_successor(&self, id: &String) -> Result<IpAddr, HandlerError> {
        let pred = self.find_predecessor(id).await?;
        let successor_ip = get_req(pred, "/successor").await?;
        Ok(successor_ip.parse()?)
    }

    async fn find_predecessor(&self, id: &String) -> Result<IpAddr, HandlerError> {
        let mut n_dash = self.self_ip;
        loop {
            let n_dash_hash = get_identifier(&n_dash.to_string());
            let successor = get_req(n_dash, "/successor").await?;
            let successor_hash = get_identifier(&successor);
            let interval =
                Interval::new(Bracket::Open, n_dash_hash, successor_hash, Bracket::Closed);
            if interval.contains(get_identifier(&id)) {
                break;
            }
            n_dash = get_req(n_dash, &format!("/successor/cpf/{}", id))
                .await?
                .parse()?;
        }

        Ok(n_dash)
    }

    pub fn closest_preceding_finger(&self, id: &String) -> IpAddr {
        let interval = Interval::new(
            Bracket::Open,
            get_identifier(&self.self_ip.to_string()),
            get_identifier(id),
            Bracket::Open,
        );
        for entry in self.finger_table.lock().unwrap().finger_table.iter().rev() {
            if interval.contains(entry.successor) {
                return entry.node_ip;
            }
        }
        return self.self_ip;
    }
}

pub fn initialize_node() -> ChordNode {
    let args: Vec<String> = env::args().collect();
    let mut finger_table = FingerTable::new();
    let hash_map = HashMap::new();
    let self_ip = get_self_ip();
    let self_id = get_identifier(&self_ip.to_string());
    println!("My ip is {} and my ID is {}", self_ip, self_id);
    if args.len() == 1 {
        // first node
        let start = (self_id + 1) % M;
        let interval = Interval::new(Bracket::Closed, start, start, Bracket::Closed);
        let first_entry = FingerTableEntry::new(start, interval, self_id, self_ip);
        finger_table.add_entry(first_entry);
    } else {
        // contact the node to:
        // 1. update its successor
        // 2. transfer its keys to this node
        unimplemented!()
    }

    ChordNode::new(finger_table, hash_map, self_ip)
}

async fn get_req(ip: IpAddr, path: &str) -> Result<String, HandlerError> {
    let resp = reqwest::get(format!("http://{}:{}{}", ip, PORT, path)).await?;
    Ok(resp.text().await?)
}

fn get_self_ip() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("8.8.8.8:80").unwrap();
    socket.local_addr().unwrap().ip()
}

fn get_identifier(key: &String) -> u64 {
    fn hasher(key: &String) -> u64 {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        s.finish()
    }
    hasher(key) % M
}
