# crust

Authors:

- Soham Dongargaonkar
- Gagan Hegde

## Summary Description

[Distributed Hash Tables](https://en.wikipedia.org/wiki/Distributed_hash_table) offer key-value lookups corresponding to a Hash Table that is "split" across several nodes.
DHTs follow a "ring" based topology and aim to achieve the following goals:
 - Scalability: Allow quick lookups (`log(n)` time, where n is the number of participating nodes in the distributed system).
 - Fault Tolerance: Allow nodes to join at any time, leave or crash at any time.
 - Decentralization: No node is in "control" of the system. All nodes are of equal priority.

## Use cases
crust aims to be an implementation of the [Chord](https://en.wikipedia.org/wiki/Chord_(peer-to-peer)) protocol. Chord provides the core idea for implementing a Distributed Hash Table: 
```
... given a key, [Chord] will determine the node responsible for storing the key’s value.
```
Note that the DHT should be completely "transparent" in that a client using the DHT will have no idea if the underlying structure is a Hash Map or a Distributed Hash Map. The system should allow the client to simply query the keys as they would query a normal Hash Map. 

## Possible Components:
- `struct Node` encapsulates a node that provides the Hash Map functionality:
    - pub fn get(key) -> Option\<T\>
    - pub fn insert(key, value) -> bool
    - pub fn remove(key) -> Option\<T\>
    - fn forward_request(node, key) -> Option\<T\>

## Thoughts on Testing
- Use TDD (Test Driven Development) throughout.
- Challenges:
    - Creating the Chord ring topology in the test environment
    - Creating network partitions
    - Creating large number of nodes
    - Polling adjacent nodes
    - Crashing nodes intentionally and updating adjacent nodes to represent the new state of the ring.
    - Querying nodes at random
    - Trying to acheive O(log n) turnaround time

## Thoughts on MVP
- Ring topology with successful lookups, **but `O(n)` instead of `log(n)`** (if key not present at node, the node simply forwards request to the next node)
- Tests for insert, get, remove. 

## Stretch Goals
- Full Chord implementation with handling of failures and dynamic node joinings. Lookup must be **log(n)**
- Robust test suite
- Introduce real life scenario with applications based off numerous geographical locations able to share data with distributed hash maps.
- Demonstrate an end-to-end test automated through CI.

## Functionality to be completed at Checkpoint
- Since setting up the test environment seems to be a non trivial task, focus should be to get the environment up and working first.
- As a first checkpoint, the project should have concrete tests that create the Ring topology and can randomly query or drop nodes.
