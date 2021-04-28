# crust

Crust is an implementation of [Chord](https://en.wikipedia.org/wiki/Chord_(peer-to-peer)) in Rust.
This is a WIP. See the [projects section](https://github.com/a3y3/crust/projects/1) to see what we're working on currently (or what's next)

## What works as of now
There's no "hash map" functionality - but we have a naive version of Chord up and running that correctly has successor and predecessor pointers (no finger tables yet). 

So our `successor(n)` function works, but it runs in O(n).

## Build
`docker build . -t crust`

## Run
- To start the first node: `docker run --init --rm -p 8000:8000 crust`
- Open a browser and go to `localhost:8000/info` to see the sucessor and predecessor pointers for the first node
- To start the second node: open a new Terminal window and see the IP address from the output of the first node. For example, if it's `172.17.0.2`, run `docker run --init --rm -p 8000:8000 crust -- 172.17.0.2`
- Open a new tab and go to `localhost:8001/info` to see the pointers for the second node and so on

Authors:

- Soham Dongargaonkar
- Gagan Hegde
