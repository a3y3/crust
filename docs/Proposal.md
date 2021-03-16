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
Note that the DHT should be completely "transparent" in that a client using the DHT will have no idea if the underlying structure is a Hash Map or a Distributed Hash Map. The system should allow the client to simply query the keys is they would query a normal Hash Map. 
