use gotham::handler::HandlerError;
use gotham_derive::StateData;
use rand::Rng;
use reqwest::Response;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::Serialize;
use serde_json;
use simple_error::SimpleError;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::{env, fmt};
use std::{thread, time::Duration};

const M: u64 = 64;
const PORT: usize = 8000;

const HTTP_SUCCESSOR: &str = "successor/";
const HTTP_SUCCESSOR_CPF: &str = "successor/cpf/";
const HTTP_PREDECESSOR: &str = "predecessor/";
const HTTP_FINGER_TABLE: &str = "fingertable/";
const HTTP_NOTIFY: &str = "notify/";
const HTTP_KEY: &str = "key/";
const HTTP_REPLICA: &str = "replica/";

const STABILIZE_INTERVAL: u64 = 2;
const LIVENESS_TIMEOUT: u64 = 1;
const REQ_TIMEOUT: u64 = 3;
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

    /// todo Needs improvement - right now this method uses a pretty inefficient way to check if a `val` lies between an `Interval. Selecting a `start` and and `end` and walking through each value between them isn't ideal, and we need  a faster way to do this.
    fn contains(&self, val: u64) -> bool {
        let mut start = match self.bracket1 {
            Bracket::Open => (self.val1 + 1) % M,
            Bracket::Closed => self.val1,
        };

        let end = match self.bracket2 {
            Bracket::Open => (self.val2 + M - 1) % M,
            Bracket::Closed => self.val2,
        };

        while start != end {
            if start == val {
                return true;
            }
            start = (start + 1) % M;
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

impl Serialize for FingerTableEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FingerTableEntry", 4)?;
        state.serialize_field("start", &self.start)?;
        state.serialize_field("interval", &format!("{}", self.interval))?;
        state.serialize_field("successor_id", &self.successor)?;
        state.serialize_field("successor", &self.node_ip)?;

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
struct VisInfo {
    from: u64,
    to: u64,
}

impl VisInfo {
    fn new(from: u64, to: u64) -> Self {
        Self { from, to }
    }
}
#[derive(Clone, StateData)]
pub struct ChordNode {
    finger_table: Arc<Mutex<Vec<FingerTableEntry>>>,
    hash_set: Arc<Mutex<HashSet<String>>>,
    self_ip: IpAddr,
    predecessor: Arc<Mutex<IpAddr>>,
    successor_list: Arc<Mutex<Vec<IpAddr>>>,
    replica_set: Arc<Mutex<HashSet<String>>>,
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
        let successor_list = self.successor_list.lock().unwrap();
        let successor_list: Vec<(&IpAddr, u64)> = successor_list
            .iter()
            .map(|ip| (ip, get_identifier(&ip.to_string())))
            .collect();
        let hash_set = self.hash_set.lock().unwrap();

        let mut state = serializer.serialize_struct("ChordNode", 5)?;
        state.serialize_field("finger_table", &*finger_table)?;
        state.serialize_field("hash_set", &*hash_set)?;
        state.serialize_field("self_ip", &self.self_ip)?;
        state.serialize_field("self_id", &self_id)?;
        state.serialize_field("predecessor", &*predecessor)?;
        state.serialize_field("predecessor_id", &predecessor_id)?;
        state.serialize_field("successor_list", &*successor_list)?;
        state.end()
    }
}

impl ChordNode {
    fn new(
        finger_table: Vec<FingerTableEntry>,
        hash_set: HashSet<String>,
        self_ip: IpAddr,
        predecessor: IpAddr,
    ) -> Self {
        let finger_table = Arc::new(Mutex::new(finger_table));
        let hash_set = Arc::new(Mutex::new(hash_set));
        let predecessor = Arc::new(Mutex::new(predecessor));
        let successor_list = Arc::new(Mutex::new(Vec::new()));
        let replica_set = Arc::new(Mutex::new(HashSet::new()));
        Self {
            finger_table,
            hash_set,
            self_ip,
            predecessor,
            successor_list,
            replica_set,
        }
    }

    pub fn info(&self) -> String {
        serde_json::to_string_pretty(self).expect("Can't serialize table")
    }

    pub async fn ring_info(&self) -> Result<String, HandlerError> {
        let mut set = HashSet::new();
        let mut curr_ip = self.self_ip;
        let mut current = get_identifier(&curr_ip.to_string());
        set.insert(current);
        let mut succ_ip = self.get_successor();
        let mut successor = get_identifier(&succ_ip.to_string());
        let v = VisInfo::new(current, successor);
        let mut result = Vec::new();
        result.push(v);

        while !set.contains(&successor) {
            current = successor;
            curr_ip = succ_ip;
            succ_ip = get_req(curr_ip, HTTP_SUCCESSOR, &self).await?.parse()?;
            successor = get_identifier(&succ_ip.to_string());
            let vis = VisInfo::new(current, successor);
            result.push(vis);
            set.insert(current);
        }

        Ok(serde_json::to_string_pretty(&result).expect("Error serializing ring info"))
    }

    pub fn get_successor(&self) -> IpAddr {
        let table = self.finger_table.lock().unwrap();
        (*table).get(0).unwrap().node_ip
    }

    pub fn update_successor(&self, new_succ: IpAddr) {
        let mut table = self.finger_table.lock().unwrap();
        let prev_entry = table.get_mut(0).unwrap();
        let old_id = get_identifier(&prev_entry.node_ip.to_string());
        let new_id = get_identifier(&new_succ.to_string());
        println!("prev successor: {} (id:{})", prev_entry.node_ip, old_id);
        (*prev_entry).node_ip = new_succ;
        (*prev_entry).successor = new_id;
        println!(
            "Updated successor to {} (id:{})",
            prev_entry.node_ip, new_id
        );
    }

    pub fn get_predecessor(&self) -> IpAddr {
        *self.predecessor.lock().unwrap()
    }

    pub fn update_predecessor(&self, ip: IpAddr) {
        *self.predecessor.lock().unwrap() = ip
    }

    pub async fn calculate_successor(&self, id: &String) -> Result<IpAddr, HandlerError> {
        let id: u64 = id.parse()?;
        assert!(id < M);
        let pred = self.calculate_predecessor(id).await?;
        let successor_ip = get_req(pred, HTTP_SUCCESSOR, self).await?;
        Ok(successor_ip.parse()?)
    }

    async fn calculate_predecessor(&self, id: u64) -> Result<IpAddr, HandlerError> {
        let mut n_dash = self.self_ip;
        loop {
            let n_dash_id = get_identifier(&n_dash.to_string());
            let successor = if n_dash == self.self_ip {
                self.get_successor().to_string()
            } else {
                get_req(n_dash, HTTP_SUCCESSOR, self).await?
            };
            let successor_hash = get_identifier(&successor);
            let interval = Interval::new(Bracket::Open, n_dash_id, successor_hash, Bracket::Closed);
            if interval.contains(id) {
                break;
            }
            n_dash = if n_dash == self.self_ip {
                self.closest_preceding_finger(&id.to_string())
            } else {
                get_req(n_dash, &format!("{}{}/", HTTP_SUCCESSOR_CPF, id), self)
                    .await?
                    .parse()?
            };
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
        for entry in self.finger_table.lock().unwrap().iter() {
            if interval.contains(entry.successor) {
                return entry.node_ip;
            }
        }
        return self.self_ip;
    }

    pub async fn update_finger_table(&mut self, s: IpAddr, i: u64) -> Result<(), HandlerError> {
        let self_id = get_identifier(&self.self_ip.to_string());
        let s_id = get_identifier(&s.to_string());
        let ith_ip_id = {
            let locked_table = self.finger_table.lock().unwrap();
            (*locked_table).get(i as usize).unwrap().successor.clone()
        };
        let interval = Interval::new(Bracket::Closed, self_id, ith_ip_id, Bracket::Open);
        if interval.contains(s_id) {
            {
                let mut lock = self.finger_table.lock().unwrap();
                let entry = lock.get_mut(i as usize).unwrap();
                entry.successor = s_id;
                entry.node_ip = s;
            }
            let pred = *self.predecessor.lock().unwrap();
            let pred_id = get_identifier(&pred.to_string());
            if pred_id == s_id {
                println!("Warning: Skipping patching my predecessor because it's the same as s_id! See the README for what this warning means.");
                return Ok(());
            }
            println!("Done. Patching my predecessor ({})", pred_id);
            data_req(
                pred,
                HTTP_FINGER_TABLE,
                vec![("n", s.to_string()), ("i", i.to_string())],
                self,
                "PATCH",
            )
            .await?;
        }

        Ok(())
    }

    async fn stabilize(&self) -> Result<(), HandlerError> {
        let client = reqwest::Client::new();
        loop {
            thread::sleep(Duration::from_secs(STABILIZE_INTERVAL));
            let succ_ip = self.get_successor();
            let successors_predecessor = get_req(succ_ip, HTTP_PREDECESSOR, self).await?;
            if is_node_alive(successors_predecessor.parse()?, Some(&client)).await
                && successors_predecessor != self.self_ip.to_string()
            {
                let successors_predecessor_id = get_identifier(&successors_predecessor);
                let self_id = get_identifier(&self.self_ip.to_string());
                let succ_id = get_identifier(&succ_ip.to_string());
                let int_self_to_successor =
                    Interval::new(Bracket::Open, self_id, succ_id, Bracket::Open);
                if int_self_to_successor.contains(successors_predecessor_id) {
                    println!("stabilize() found a new successor, updating...");
                    self.update_successor(successors_predecessor.parse()?);
                }

                // notify successor that this node should be their predecessor
                data_req(
                    succ_ip,
                    HTTP_NOTIFY,
                    vec![("n", self.self_ip.to_string())],
                    self,
                    "PATCH",
                )
                .await?;
            }

            // fix fingers
            self.fix_fingers().await?;

            // rebuild successor list
            self.build_successor_list().await?;
        }
    }

    pub async fn notify(&self, other_node: IpAddr) {
        let predecessor = self.get_predecessor();
        let pred_id = get_identifier(&predecessor.to_string());
        let other_id = get_identifier(&other_node.to_string());
        let self_id = get_identifier(&self.self_ip.to_string());
        let int_predecessor_to_self = Interval::new(Bracket::Open, pred_id, self_id, Bracket::Open);
        let is_predecessor_alive = is_node_alive(predecessor, None).await;
        if !is_predecessor_alive
            || (predecessor == self.self_ip)
            || (int_predecessor_to_self.contains(other_id))
        {
            if other_node != predecessor {
                println!(
                    "notify found a better predecessor. Updating to {} (id:{})",
                    other_node,
                    get_identifier(&other_node.to_string())
                );
            }
            self.update_predecessor(other_node);
        }
    }

    async fn fix_fingers(&self) -> Result<(), HandlerError> {
        let m = (M as f64).log2() as u32;
        let rand_idx = rand::thread_rng().gen_range(0..m) as usize;

        let start = {
            let table = self.finger_table.lock().unwrap();
            let rand_entry = table.get(rand_idx).unwrap();
            rand_entry.start
        };

        let succ = self.calculate_successor(&start.to_string()).await?;
        let succ_id = get_identifier(&succ.to_string());
        {
            let mut table = self.finger_table.lock().unwrap();
            let rand_entry = table.get_mut(rand_idx).unwrap();
            rand_entry.node_ip = succ;
            rand_entry.successor = succ_id;
        }
        Ok(())
    }

    async fn build_successor_list(&self) -> Result<(), HandlerError> {
        let m = (M as f64).log2() as u32;
        let client = reqwest::Client::new();
        let mut successor = self.get_successor();
        let mut new_successors = Vec::new();
        for _ in 0..m {
            let next_successor = client
                .get(format!("http://{}:{}/{}", successor, PORT, HTTP_SUCCESSOR))
                .timeout(Duration::from_secs(REQ_TIMEOUT))
                .send()
                .await;
            match next_successor {
                Ok(s) => successor = s.text().await?.parse()?,
                //if a potential successor is down, skip adding it to the list.
                Err(_) => break,
            };
            new_successors.push(successor);
        }
        *self.successor_list.lock().unwrap() = new_successors;
        Ok(())
    }

    async fn handle_failure(&self) {
        // check if successor is alive
        println!("Failure detected, attempting to fix pointers...");
        let client = reqwest::Client::new();
        let successor_ip = self.get_successor();
        let resp = client
            .get(format!(
                "http://{}:{}/{}",
                successor_ip, PORT, HTTP_SUCCESSOR
            ))
            .timeout(Duration::from_secs(REQ_TIMEOUT))
            .send()
            .await;
        match resp {
            Ok(_) => {}
            Err(_) => {
                println!("Successor is down. Fixing...");
                let new_succ = self.get_first_live_successor(&client).await;
                self.update_successor(new_succ);
                println!(
                    "Notifying my new successor (id:{}) to update their predecessor...",
                    get_identifier(&new_succ.to_string())
                );
                client
                    .patch(format!("http://{}:{}/{}", new_succ, PORT, HTTP_NOTIFY))
                    .timeout(Duration::from_secs(REQ_TIMEOUT))
                    .form(&vec![("n", self.self_ip.to_string())])
                    .send()
                    .await
                    .unwrap();
            }
        };

        // check if predecessor is alive
        let predecessor_ip = self.get_predecessor();
        let resp = client
            .get(format!(
                "http://{}:{}/{}",
                predecessor_ip, PORT, HTTP_SUCCESSOR
            ))
            .timeout(Duration::from_secs(REQ_TIMEOUT))
            .send()
            .await;
        match resp {
            Ok(_) => {}
            Err(_) => {
                println!("Predecessor is down. Fixing to self IP.");
                self.update_predecessor(self.self_ip)
            }
        }
    }

    async fn get_first_live_successor(&self, client: &reqwest::Client) -> IpAddr {
        let entries: Vec<IpAddr> = {
            let table = self.successor_list.lock().unwrap();
            table.iter().map(|ip| *ip).collect()
        };

        for possible_succ in entries {
            println!("Trying to contact {}", possible_succ);
            let resp = client
                .get(format!(
                    "http://{}:{}/{}",
                    possible_succ, PORT, HTTP_SUCCESSOR
                ))
                .timeout(Duration::from_secs(REQ_TIMEOUT))
                .send()
                .await;
            match resp {
                Ok(_) => return possible_succ,
                Err(_) => continue,
            }
        }
        return self.self_ip;
    }

    pub async fn insert(&self, key: String) -> Result<String, HandlerError> {
        let key_id = get_identifier(&key);
        let key_successor = self.calculate_successor(&key_id.to_string()).await?;
        if key_successor == self.self_ip {
            //insert here!
            (*self.hash_set.lock().unwrap()).insert(key.clone());
            self.send_to_replicas(key).await?;
        } else {
            let inserted_at =
                data_req(key_successor, HTTP_KEY, vec![("key", key)], &self, "POST").await?;
            return Ok(inserted_at);
        }
        Ok(self.self_ip.to_string())
    }

    async fn send_to_replicas(&self, key: String) -> Result<(), HandlerError> {
        let list = self.successor_list.lock().unwrap().clone();
        for node in list {
            data_req(
                node,
                HTTP_REPLICA,
                vec![("key", key.clone())],
                &self,
                "POST",
            )
            .await?;
        }
        Ok(())
    }

    pub fn insert_replica(&self, key: String) {
        (*self.replica_set.lock().unwrap()).insert(key);
    }

    pub async fn contains(&self, key: &String) -> Result<bool, HandlerError> {
        let key_id = get_identifier(key);
        let key_successor = self.calculate_successor(&key_id.to_string()).await?;
        if key_successor == self.self_ip {
            // this node is responsible for this key!
            match (*self.hash_set.lock().unwrap()).contains(key) {
                true => return Ok(true),
                false => match (*self.replica_set.lock().unwrap()).contains(key) {
                    true => {
                        println!("Warning: Key found, but in replica set. This means this node is now the new owner of this key (as opposed to being just a replica). This key should be moved from replica set to hash set.");
                        return Ok(true);
                    }
                    false => {
                        return Ok(false);
                    }
                },
            }
        } else {
            // this node isn't responsible, contact key_successor.
            let result: bool = get_req(key_successor, &format!("{}{}", HTTP_KEY, key), self)
                .await?
                .parse()?;
            Ok(result)
        }
    }
}

pub fn initialize_node() -> ChordNode {
    let args: Vec<String> = env::args().collect();
    let self_ip = get_self_ip();
    let self_id = get_identifier(&self_ip.to_string());
    println!("My ip is {} and my ID is {}", self_ip, self_id);
    if args.len() == 1 {
        // first node
        let mut finger_table = Vec::new();
        let hash_map = HashSet::new();
        let m = (M as f64).log2() as u32;
        for i in 0..m {
            let start = get_start(self_id, i);
            let k_plus_one_start = get_start(self_id, i + 1);
            let interval = Interval::new(Bracket::Closed, start, k_plus_one_start, Bracket::Open);
            let first_entry = FingerTableEntry::new(start, interval, self_id, self_ip);
            finger_table.push(first_entry);
        }

        return ChordNode::new(finger_table, hash_map, self_ip, self_ip);
    } else {
        let node = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(join(self_ip, args[1].parse().unwrap()))
            .unwrap();
        node
    }
}

fn get_start(n: u64, k: u32) -> u64 {
    (n + u64::pow(2, k)) % M
}

async fn join(self_ip: IpAddr, existing_node: IpAddr) -> Result<ChordNode, HandlerError> {
    println!("Initializing my finger tables...");
    let node = init_finger_table(self_ip, existing_node).await?;
    println!("Done.");
    println!("Skipping moving keys...");
    move_keys().await?;
    Ok(node)
}

async fn init_finger_table(
    self_ip: IpAddr,
    existing_node: IpAddr,
) -> Result<ChordNode, HandlerError> {
    let self_id = get_identifier(&self_ip.to_string());
    let m = (M as f64).log2() as u32;
    let mut finger_table = Vec::new();
    for i in 0..m {
        let start = get_start(self_id, i);
        let start_plus_one = get_start(self_id, i + 1);
        let interval = Interval::new(Bracket::Closed, start, start_plus_one, Bracket::Open);
        let succ_ip = if i == 0 {
            let succ_ip = reqwest::get(format!(
                "http://{}:{}/{}",
                existing_node, PORT, HTTP_SUCCESSOR
            ))
            .await?
            .text()
            .await?;
            println!(
                "My successor is {} (id:{})",
                succ_ip,
                get_identifier(&succ_ip.to_string())
            );
            succ_ip
        } else {
            self_ip.to_string()
        };
        let succ_id = get_identifier(&succ_ip.to_string());
        let entry = FingerTableEntry::new(start, interval, succ_id, succ_ip.parse()?);
        finger_table.push(entry);
    }

    let predecessor = self_ip;
    println!("Setting my predecessor as me. This will be fixed later by notify()");

    Ok(ChordNode::new(
        finger_table,
        HashSet::new(),
        self_ip,
        predecessor,
    ))
}

async fn move_keys() -> Result<(), HandlerError> {
    Ok(())
}

async fn get_req(ip: IpAddr, path: &str, chord_node: &ChordNode) -> Result<String, HandlerError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}:{}/{}", ip, PORT, path))
        .timeout(Duration::from_secs(REQ_TIMEOUT))
        .send()
        .await;
    let resp = match resp {
        Ok(resp) => resp,
        Err(e) => {
            chord_node.handle_failure().await;
            return Err(e.into());
        }
    };
    let text = request_unsuccessful(resp, "GET").await?;
    return Ok(text);
}

async fn data_req<T, U>(
    ip: IpAddr,
    path: &str,
    data: Vec<(T, U)>,
    chord_node: &ChordNode,
    req_type: &str,
) -> Result<String, HandlerError>
where
    T: Serialize + Sized,
    U: Serialize + Sized,
{
    let client = reqwest::Client::new();
    let response = match req_type {
        "PATCH" => {
            client
                .patch(format!("http://{}:{}/{}", ip, PORT, path))
                .timeout(Duration::from_secs(REQ_TIMEOUT))
                .form(&data)
                .send()
                .await
        }
        "POST" => {
            client
                .post(format!("http://{}:{}/{}", ip, PORT, path))
                .timeout(Duration::from_secs(REQ_TIMEOUT))
                .form(&data)
                .send()
                .await
        }
        "DELETE" => {
            client
                .delete(format!("http://{}:{}/{}", ip, PORT, path))
                .timeout(Duration::from_secs(REQ_TIMEOUT))
                .form(&data)
                .send()
                .await
        }
        _ => {
            panic!("That's not a valid request type")
        }
    };
    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            chord_node.handle_failure().await;
            return Err(e.into());
        }
    };

    let text = request_unsuccessful(response, req_type).await?;
    Ok(text)
}

async fn request_unsuccessful(response: Response, req_type: &str) -> Result<String, HandlerError> {
    let status = response.status();
    if status != 200 {
        let error = SimpleError::new(format!(
            "Received error from {} req: {}",
            req_type,
            response.text().await?
        ));
        let handler_error = HandlerError::from(error).with_status(status);
        return Err(handler_error);
    }
    Ok(response.text().await?)
}

async fn is_node_alive(ip: IpAddr, client: Option<&reqwest::Client>) -> bool {
    let client = match client {
        Some(client) => client.clone(),
        None => reqwest::Client::new(),
    };

    let response = client
        .get(format!("http://{}:{}/{}", ip, PORT, HTTP_SUCCESSOR))
        .timeout(Duration::from_secs(LIVENESS_TIMEOUT))
        .send()
        .await;
    match response {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn start_stabilize_thread(chord_node: ChordNode) {
    thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(err_stabilize(chord_node));
    });
}

async fn err_stabilize(chord_node: ChordNode) {
    loop {
        match chord_node.stabilize().await {
            Ok(()) => {}
            Err(_e) => {
                println!("Warning: Retrying stabilize() to see if the issue was fixed automatically (there's a good chance it was). If you see this warning more than 5-6 times in a row, exit the program and debug - but if you see nothing after this, the issue was likely self-fixed.")
            }
        }
    }
}

fn get_self_ip() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("8.8.8.8:80").unwrap();
    socket.local_addr().unwrap().ip()
}

pub fn get_identifier(key: &String) -> u64 {
    fn hasher(key: &String) -> u64 {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        s.finish()
    }
    hasher(key) % M
}
