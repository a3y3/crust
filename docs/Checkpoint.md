# crust

Team members:

- Soham Dongargaonkar
- Gagan Hegde

## Summary Description
**crust** ([Chord](https://en.wikipedia.org/wiki/Chord_(peer-to-peer)) in Rust) is an implementation of Chord, a Distributed Hash Table protocol.

[Quick Reminder] A Distributed Hash Table is a HashMap that is split across multiple nodes. In the system, nodes are constantly joining and leaving/failing, but we should still have a `O(log n)` key lookup (where n is the number of nodes in the system). 

## Checkpoint Progress Summary
Since the proposal, our focus has been to develop a way to:
- Find a way to represent each "node" in the Distributed Hash Table.
  - We have settled with docker - each node is a container.
  - For creating multiple containers, we can either use multiple Terminal instances, or `docker-compose`.
- Find a way to communicate between 2 nodes (if node-x doesn't have a particular key, how does it communicate with node-y?)
  - We initially started out by assuming that each node could just use a `TCPListener`. However, this raises an important question: how should the client interact with the system? How should they query the system for a key? 
  - We settled on using HTTP to communicate between and to and fro nodes. This offers an elegant solution to the previous question: a node just listens to an incoming request. The request could come from either the client, or another node that didn't have the particular key.
  - After using the popular backend framework [Actix-Web](https://actix.rs/) we increasingly became tired at the long compile times - every single small change that required a `cargo build` took 30+ seconds to compile!
  - We researched and played around with a few more frameworks and finally settled on [Gotham](https://gotham.rs/) - a lightweight framework that takes around 8 seconds to compile - which is still a lot, but much more acceptable.

- Find a way to do Test Driven Development
  - If we are using docker, how do we set up containers and still remain in the Rust environment (and have the power to do `assert_eq!(response, expected_response)`)? How do we create and shut down containers at will? How do we communicate with a particular container (which in itself, is a node)?
  - We used a crate called [dockertest](https://docs.rs/dockertest/0.2.1/dockertest/). The crate is designed to create containers from within a Rust test environment, and they implment `Drop` - which means shutting down containers is very easy! After running into many, many (and many) issues with it and talking to the developers, we finally managed to get it to work.
 
Additionally, we have read and understood the paper itself.


## Next steps
- Implement a naive version of the protocol that queries for a key in `O(n)`. 
- Generalize the key-value operations (right now we just have String-String)
- Implement the full Chord protocol which queries a key in `O(log n)`
