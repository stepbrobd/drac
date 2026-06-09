# [Drac](https://en.wikipedia.org/wiki/Drac_(river))

I want a single binary named after a evil dragon. Working on this for fun ;)

- Auth DNS
  - https://github.com/hickory-dns/hickory-dns
  - https://github.com/nlnetlabs/domain
- Layer 7 load balancing, caching, proxy, rewrite
  - https://github.com/cloudflare/pingora
  - https://github.com/foyer-rs/foyer
- Mesh VPN
  - https://github.com/nickcao/ranet
- Routing daemon with traffic steering and monitoring
  - https://github.com/holo-routing/holo
  - https://github.com/oxidecomputer/maghemite
  - https://github.com/stepbrobd/rfm
- Replicated control plane, cluster membership, health check
  - https://github.com/tikv/raft-rs
  - https://github.com/databendlabs/openraft
  - https://github.com/zarbafian/gossip
  - https://github.com/quickwit-oss/chitchat
  - https://github.com/caio/foca
  - https://github.com/etcd-io/etcd
- Inline spec checking?
  - https://github.com/verus-lang/verus
  - https://github.com/model-checking/kani

## Model?

Event driven? When the daemon (does not take flags or environment variables)
starts up, it binds to a socket (at a predetermined path, if exist, just panic)
and does nothing until CLI talks with it. CLI should only check the
predetermined path for socket (error if not exist) and ask the daemon to read a
config file, or reconfigure based on the config file path passed (maybe the
daemon should keep a internal reference of the "generation" of the config
passed, any mutation of the config thru API or CLI will result in a new
generation and the changes should be synchoized with other nodes).

This implies we should aim for global consensus (but hard no? consensus with
what bound? eventual consistency? or enforce a stronger bounded model?), and
some nodes can be configured as a reflector (cue BIRD RR), and slave nodes
behind RR only need to know the master node's config (or this can also be
skipped entirely) and its own config (useful for doing load balancing).

## Configuration?

A readable version for human (I think I'll go with TOML for now cause this
should be a format supported natively by Nix without IFD) and a easily
serializable format for machines (JSON for now cause its also natively supported
by Nix)?

Config classes? Read only config? Runtime modifiable config (e.g. DNS zone
transfer) with initial entries (e.g. BGP peers)? Volatile data (e.g. Babel
entries)?
