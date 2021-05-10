# crust 

crust an implementation of the [Chord algorithm](https://en.wikipedia.org/wiki/Chord_(peer-to-peer)) in Rust.

#### Authors
- Soham Dongargaonkar
- Gagan Hegde

## Summary
The Chord algorithm provides an ability to only a single task:

```
... given a key, [Chord] will determine the node responsible for storing the key’s value.
```
Thus, the only thing that Chord is responsible for is returning the IP address of a node where a key should be stored. What to do with this information is entirely up to the application using Chord. 

## Project Goal
- Understand the Chord algorithm and implement it in Rust.
- Use the Chord algorithm to develop a ~Distributed Hash Map~ Distributed Hash Set (we ran out of time and pivoted to the marginally simpler data structure)

## Project Structure
Our code is separated in 2 parts:
1. `main.rs` contains the Gotham backend. We modelled the Chord nodes such that every node is a full fledged web server running on an IP address and a port. 
1. `lib.rs` contains the core Chord logic. 
This design allowed to express very cleanly the philosophy of Chord itself; specifically, about Chord being an "underlying" algorithm and having an application "using" Chord to implement custom software (in our case, a Distributed Hash Set)

## Things that were quite Rust-y
- Our main problem was how to have an in-memory data structure shared by all HTTP functions (that don't belong to a struct). Fortunately Gotham provides a variable called `state` that is cloned and passed everytime an HTTP function is called.
- How to make use of this? By wrapping all fields in the struct in `Arc<Mutex<_>>`! This makes cloning efficient, and also allows all functions to have access to the same structure **safely**:
```
pub struct ChordNode {
    finger_table: Arc<Mutex<Vec<FingerTableEntry>>>,
    hash_set: Arc<Mutex<HashSet<String>>>,
    self_ip: IpAddr,
    predecessor: Arc<Mutex<IpAddr>>,
    successor_list: Arc<Mutex<Vec<IpAddr>>>,
    replica_set: Arc<Mutex<HashSet<String>>>,
}
```
- At the start of the project we didn't plan for the fact that apart from using `async`/`await`, we'd also have to create separate threads for stabilizing the Chord ring periodically. However, because all of our fields were already wrapped in `Arc<Mutex<_>>`, we had to change absolutely nothing in the core logic!
- Another Rust-y thing is that when a node is "responsible" for a key, it quite literally is the owner of the key. When sending replicas of a key to nodes, a node has to literally create replicas of the key - by calling `.clone()` on the key and then sending that replica to the other successors.


## Things that were not so Rust-y
- We had to create 2 functions for sending requests (for GET and PATCH/POST) that essentially do the same thing, but still exist as 2 separate methods. This is because of a `data` argument in the PATCH request that the GET request doesn't have.
```
async fn data_req<T, U>(ip: IpAddr, path: &str, data: Vec<(T, U)>) -> Result<String, HandlerError>
where
    T: Serialize + Sized,
    U: Serialize + Sized,
{
```
If we wrap the `data` argument in an `Option` and send `None` while sending a GET request, Rust throws a compilation error because the type parameters for `T,U` cannot be inferred. We're sure there's a better way of fixing this rather than having code repetition, but we couldn't figure out a way. 

## Design mistakes we made
- Although the initial goal was to write code using TDD(test-driven-development), we faced a few problems in our endevour. We list a few reasons why this was harder than anticipated. Due to lack of time we could not use the test crates to write standalone unit test cases (e.g `gotham::test` to test valid API end-points). Some functionality within the code reflected on changes on the UI-front or from within a different docker-container, by the likes of how chord works made this a hard problem because we cant signal to dockertest to start a new container after handing over a list of containers to be started by `dockertest`. Similarly look ups might fail on occasions since its hard to determine when the `stabilize()` method has updated the finger table. The last problem was the end-to-end tests, initailly we did not expose endpoints to add a node and delete a node from the chord network, so as previosuly stated we could not create a new-container after signalling to `dockertest` a list of containers to be created, however once the end-points were exposed these test cases could have been asserted based on the returned JSON. One way we could have avoided this problem is to have started by writting test cases to derive functions and not the other way round.
- We should have used gRPC to model the chord network as an RPC server instead of a HTTP server. The server way introduced a lot of unnecessary verbosity. For example, `n.update_successor(n1)` in gRPC vs creating and sending a PATCH request, and handling errors manually in that request vs having the gRPC framework do it for us. 
- Our design made it so that all HTTP requests are sent through only 2 functions. In these functions, if a request times out, or is rejected, the destination node IP is marked as failed and failure recovery is triggered. Unfortunately, this design doesn't allow us to treat different failures differently. Failures that originate from `stabilize()` function should be treated differently than failures that originate while calculating the successor of a node (in that, the user should be informed that they should re-try the query again, instead of triggering failure recovery - which will be triggered automatically by the every-running `stabilize()` method).
- The Chord paper specifies 2 algorithms; the first is an aggressive way of maintaining the state of the Chord ring and doesn't handle failures or concurrent node joins. The second is a much simpler, less sophisticated algorithm that does. However, we didn't realize this (we just read the Chord paper until the first algorithm, implemented that, and then moved on to the second algorithm). This meant having to erase a lot of the code we worked so hard on. We could have saved ourselves a lot of time had we read the entire paper first, and then started with implementation. Although: a silver lining of this was that we could create [release tags](https://github.com/a3y3/crust/releases) for both these algorithms and showcase how they differ!
