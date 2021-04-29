use gotham::handler::HandlerError;
use gotham_derive::StateData;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::Serialize;
use serde_json;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, UdpSocket};
use std::sync::{Arc, Mutex};

const M: u64 = 64;
const PORT: usize = 8000;

const HTTP_SUCCESSOR: &str = "successor/";
const HTTP_SUCCESSOR_CFP: &str = "successor/cfp/";
const HTTP_PREDECESSOR: &str = "predecessor/";

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

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first_bracket = match self.bracket1 {
            Bracket::Open => '(',
            Bracket::Closed => '[',
        };
        let second_bracket = match self.bracket2 {
            Bracket::Open => ')',
            Bracket::Closed => ']',
        };
        write!(
            f,
            "{}{},{}{}",
            first_bracket, self.val1, self.val2, second_bracket
        )
    }
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

#[allow(dead_code)] //todo remove this later
struct FingerTableEntry {
    start: u64,
    interval: Interval,
    successor: u64,
    node_ip: IpAddr,
}

impl Serialize for FingerTableEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Interval", 4)?;
        state.serialize_field("start", &self.start)?;
        state.serialize_field("interval", &format!("{}", self.interval))?;
        state.serialize_field("successor", &self.successor)?;
        state.serialize_field("node_ip", &self.node_ip)?;

        state.end()
    }
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

#[derive(Serialize)]
struct FingerTable {
    fingers: Vec<FingerTableEntry>,
}

impl FingerTable {
    fn new() -> Self {
        FingerTable {
            fingers: Vec::new(),
        }
    }

    fn add_entry(&mut self, entry: FingerTableEntry) {
        self.fingers.push(entry);
    }
}

#[derive(Clone, StateData)]
pub struct ChordNode {
    finger_table: Arc<Mutex<FingerTable>>,
    hash_map: Arc<Mutex<HashMap<String, String>>>,
    self_ip: IpAddr,
    predecessor: Arc<Mutex<IpAddr>>,
}

impl Serialize for ChordNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let finger_table = self.finger_table.lock().unwrap();
        let self_id = get_identifier(&self.self_ip.to_string());
        let predecessor = self.predecessor.lock().unwrap();
        let predecessor_id = get_identifier(&(*predecessor).to_string());

        let mut state = serializer.serialize_struct("Interval", 4)?;
        state.serialize_field("finger_table", &*finger_table)?;
        state.serialize_field("hash_map", &"fixme: unimplemented!()")?;
        state.serialize_field("self_ip", &self.self_ip)?;
        state.serialize_field("self_id", &self_id)?;
        state.serialize_field("predecessor", &*predecessor)?;
        state.serialize_field("predecessor_id", &predecessor_id)?;
        state.end()
    }
}

impl ChordNode {
    fn new(
        finger_table: FingerTable,
        hash_map: HashMap<String, String>,
        self_ip: IpAddr,
        predecessor: IpAddr,
    ) -> Self {
        let finger_table = Arc::new(Mutex::new(finger_table));
        let hash_map = Arc::new(Mutex::new(hash_map));
        let predecessor = Arc::new(Mutex::new(predecessor));
        Self {
            finger_table,
            hash_map,
            self_ip,
            predecessor,
        }
    }

    pub fn info(&self) -> String {
        serde_json::to_string_pretty(self).expect("Can't serialize table")
    }

    pub fn get_successor(&self) -> IpAddr {
        let table = self.finger_table.lock().unwrap();
        (*table).fingers.get(0).unwrap().node_ip
    }

    pub fn update_successor(&mut self, new_succ: IpAddr) {
        let mut table = self.finger_table.lock().unwrap();
        let prev_entry = table.fingers.get_mut(0).unwrap();
        let new_id = get_identifier(&new_succ.to_string());
        println!("prev succ: {}", prev_entry.node_ip);
        (*prev_entry).node_ip = new_succ;
        (*prev_entry).successor = new_id;
        println!("Updated succ to {} (id:{})", prev_entry.node_ip, new_id);
    }

    pub fn get_predecessor(&self) -> IpAddr {
        *self.predecessor.lock().unwrap()
    }

    pub fn update_predecessor(&mut self, ip: IpAddr) {
        *self.predecessor.lock().unwrap() = ip
    }

    pub async fn calculate_successor(&self, id: &String) -> Result<IpAddr, HandlerError> {
        let id: u64 = id.parse()?;
        assert!(id < M);
        let pred = self.find_predecessor(id).await?;
        let successor_ip = get_req(pred, HTTP_SUCCESSOR).await?;
        Ok(successor_ip.parse()?)
    }

    async fn find_predecessor(&self, id: u64) -> Result<IpAddr, HandlerError> {
        let mut n_dash = self.self_ip;
        loop {
            let n_dash_id = get_identifier(&n_dash.to_string());
            let successor = get_req(n_dash, HTTP_SUCCESSOR).await?;
            let successor_hash = get_identifier(&successor);
            let interval = Interval::new(Bracket::Open, n_dash_id, successor_hash, Bracket::Closed);
            if interval.contains(id) {
                break;
            }
            n_dash = get_req(n_dash, &format!("{}{}/", HTTP_SUCCESSOR_CFP, id))
                .await?
                .parse()?;
        }

        Ok(n_dash)
    }

    pub fn closest_preceding_finger(&self, id: &String) -> IpAddr {
        let id: u64 = id.parse().unwrap();
        assert!(id < M);
        let interval = Interval::new(
            Bracket::Open,
            get_identifier(&self.self_ip.to_string()),
            id,
            Bracket::Open,
        );
        for entry in self.finger_table.lock().unwrap().fingers.iter().rev() {
            println!("Checking if {} is in {}", entry.successor, interval);
            if interval.contains(entry.successor) {
                return entry.node_ip;
            }
        }
        return self.self_ip;
    }
}

pub fn initialize_node() -> ChordNode {
    let args: Vec<String> = env::args().collect();
    let self_ip = get_self_ip();
    let self_id = get_identifier(&self_ip.to_string());
    println!("My ip is {} and my ID is {}", self_ip, self_id);
    if args.len() == 1 {
        // first node
        let mut finger_table = FingerTable::new();
        let hash_map = HashMap::new();
        let m = (M as f64).log2() as u32;
        for i in 0..m{
            let start = get_start(self_id, i);
            let k_plus_one_start = get_start(self_id, i+1);
            let interval = Interval::new(Bracket::Closed, start, k_plus_one_start, Bracket::Open);
            let first_entry = FingerTableEntry::new(start, interval, self_id, self_ip);
            finger_table.add_entry(first_entry);
        }

        return ChordNode::new(finger_table, hash_map, self_ip, self_ip);
    } else {
        println!("Initializing node...");
        join(self_ip, args[1].parse().unwrap())
    }
}

fn get_start(n: u64, k: u32) -> u64 {
    (n + u64::pow(2, k)) % M
}

fn join(self_ip: IpAddr, existing_node: IpAddr) -> ChordNode {
    let node = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(init_finger_table(self_ip, existing_node))
        .unwrap();
    node
}

async fn init_finger_table(
    self_ip: IpAddr,
    existing_node: IpAddr,
) -> Result<ChordNode, HandlerError> {
    let self_id = get_identifier(&self_ip.to_string());
    let start = get_start(self_id, 0);
    let start_plus_one = get_start(self_id, 1);
    let interval = Interval::new(Bracket::Closed, start, start_plus_one, Bracket::Open);
    let successor = get_req(existing_node, &format!("{}{}/", HTTP_SUCCESSOR, start)).await?;
    println!("My successor is {}", successor);
    let first_entry = FingerTableEntry::new(
        start,
        interval,
        get_identifier(&successor),
        successor.parse()?,
    );
    let mut finger_table = FingerTable::new();
    finger_table.add_entry(first_entry);
    let predecessor = get_req(successor.parse()?, HTTP_PREDECESSOR).await?;
    println!("My predecessor is {}", predecessor);
    patch_req(successor.parse()?, HTTP_PREDECESSOR, vec![("ip", self_ip)]).await?;
    println!("Patched my successor so that its predecessor is me");
    patch_req(predecessor.parse()?, HTTP_SUCCESSOR, vec![("ip", self_ip)]).await?;
    println!("Patched my predecesssor so that its successor is me");

    Ok(ChordNode::new(
        finger_table,
        HashMap::new(),
        self_ip,
        predecessor.parse()?,
    ))
}

async fn get_req(ip: IpAddr, path: &str) -> Result<String, HandlerError> {
    let resp = reqwest::get(format!("http://{}:{}/{}", ip, PORT, path)).await?;
    Ok(resp.text().await?)
}

async fn patch_req<T, U>(ip: IpAddr, path: &str, data: Vec<(T, U)>) -> Result<(), HandlerError>
where
    T: Serialize + Sized,
    U: Serialize + Sized,
{
    let client = reqwest::Client::new();
    client
        .patch(format!("http://{}:{}/{}", ip, PORT, path))
        .form(&data)
        .send()
        .await?;
    Ok(())
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
