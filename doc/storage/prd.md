# TrenchDB Distributed Routing Layer Design

## Overview

TrenchDB is a decentralized, region-aware distributed database designed to operate without leader nodes or centralized metadata servers. Every node functions as a storage node, participates in routing, and contributes to maintaining the network topology.

The routing layer is responsible for locating the storage node responsible for a given object while ensuring:

* No centralized coordinator
* Logarithmic lookup complexity
* Automatic node discovery
* Self-healing routing tables
* Regional data sovereignty
* Fault tolerance through replication

---

# Design Goals

## Primary Goals

* Fully decentralized routing
* No leader or coordinator nodes
* O(log N) object lookup
* Automatic network growth
* Efficient node discovery
* Low routing table memory footprint
* High availability
* Regional data isolation

## Non-Goals

* Global metadata server
* Master-slave architecture
* Complete cluster membership on every node
* Broadcast-based routing

---

# Regional Architecture

TrenchDB separates data logically by region.

```
                    Global Network

      ┌──────────────┬──────────────┬──────────────┐
      │              │              │              │
    India         Europe        China      North America
      │              │              │              │
   Regional DHT   Regional DHT   Regional DHT   Regional DHT
```

Each region is an independent Distributed Hash Table (DHT).

Data never crosses regional boundaries.

Every node belongs to exactly one region.

---

# Object Identifier

Each stored object contains a globally unique identifier.

```
+------------+----------------------+
| Region ID  | Object ID            |
+------------+----------------------+
```

Example

```
IND-6f21b97ab3d8...
```

The Region ID determines which regional DHT owns the object.

The Object ID is hashed for placement inside that region.

---

# Node Identity

Each node owns a permanent identity.

```
NodeID = SHA256(Node Public Key)
```

Benefits

* Globally unique
* Cryptographically verifiable
* No central allocation
* Uniform distribution

---

# Object Placement

Object placement is deterministic.

```
Placement Hash

SHA256(ObjectID)
```

Every node computes the same placement hash.

The node whose ID is closest to the placement hash becomes the primary owner.

```
Object

↓

SHA256(ObjectID)

↓

Primary Owner
```

---

# Replication

Each object is replicated to multiple neighboring nodes in the keyspace.

Example

```
Replication Factor = 10

Primary

↓

Replica 1

↓

Replica 2

↓

...

↓

Replica 9
```

Neighboring means adjacent in the DHT keyspace rather than physical network topology.

---

# Routing Layer

Every node maintains a routing table.

The routing table does **not** contain every node.

Instead it stores carefully selected peers distributed across different XOR distance ranges.

```
Bucket 0

Bucket 1

Bucket 2

...

Bucket 255
```

Each bucket stores a small number of peers (for example, 20).

This allows routing tables to remain compact while still supporting logarithmic routing.

---

# XOR Distance

Distance between nodes is calculated using XOR.

```
Distance

NodeID XOR TargetID
```

The node with the smallest XOR distance is considered the closest.

Example

```
Node

10010110

Target

10010111

Distance

00000001
```

Smaller XOR values indicate closer nodes.

---

# Bucket Organization

Each bucket represents a distance range.

Example (8-bit illustration)

```
Bucket 0

Distance 1

Bucket 1

Distance 2–3

Bucket 2

Distance 4–7

Bucket 3

Distance 8–15

...

Bucket 7

Distance 128–255
```

Real deployments use 256-bit identifiers, resulting in 256 buckets.

Each bucket stores only a fixed number of peers.

---

# Bootstrapping

A new node initially knows only one or more bootstrap peers.

```
New Node

↓

Bootstrap Peer

↓

Initial Peer List
```

Bootstrap peers return a list of known nodes.

The joining node computes XOR distances and inserts those peers into the appropriate routing buckets.

---

# Routing Table Expansion

Routing tables continuously improve over time.

Every lookup returns additional peer information.

```
Node A

↓

Node B

↓

Closest Peers

↓

Routing Table Update
```

Newly discovered peers are inserted into buckets based on XOR distance.

No broadcast is required.

The network naturally converges toward an efficient topology.

---

# Routing Algorithm

When a client requests an object:

```
Client

↓

Any Known Node

↓

Determine Region

↓

Known Regional Peer

↓

DHT Lookup

↓

Primary Owner

↓

Return Object
```

Each routing hop selects the peer whose Node ID is closest to the target placement hash.

Every hop reduces the remaining search space.

Typical lookup complexity is O(log N).

---

# Bucket Maintenance

Routing tables are continuously refreshed.

Each bucket stores metadata such as

* Last refresh time
* Peer latency
* Successful lookups
* Failure count

Example

```
Bucket

Last Refresh

Lookup Count

Failures

Peers
```

Background maintenance periodically refreshes stale buckets.

---

# Peer Health

Nodes periodically verify stored peers.

```
Ping Peer

↓

Alive

Keep

↓

Dead

Remove

↓

Replace with Newly Discovered Peer
```

Routing tables automatically heal as nodes join and leave.

---

# Replica Reads

During lookup, if the primary owner is unavailable:

```
Primary

↓

Unavailable

↓

Replica 1

↓

Replica 2

↓

Replica 3
```

The first healthy replica serves the request.

This provides high availability during failures.

---

# Future Read Quorum

Future versions may support quorum reads.

Example

```
Replication Factor = 10

Read Quorum = 6

Write Quorum = 6
```

The newest object version is selected from the quorum.

Background read repair synchronizes stale replicas.

---

# Routing Table Growth

The routing table is never downloaded in full.

Instead it evolves through:

* Bootstrapping
* Object lookups
* Periodic bucket refresh
* Peer discovery
* Failure detection

This keeps routing tables lightweight while allowing the network to scale to millions of nodes.

---

# Periodic Maintenance

Each node executes background maintenance tasks.

Example schedule

```
Every 10 minutes

Refresh random routing buckets

Every hour

Verify peer health

Every few hours

Refresh distant buckets

Continuously

Update routing tables from lookup responses
```

These tasks maintain an accurate routing topology without network-wide broadcasts.

---

# Scalability Characteristics

| Property              | Complexity          |
| --------------------- | ------------------- |
| Object Lookup         | O(log N)            |
| Routing Table Size    | O(log N) buckets    |
| Bucket Size           | Fixed (k peers)     |
| Memory Usage          | Constant per bucket |
| Node Join             | Localized           |
| Node Failure Recovery | Automatic           |

---

# Design Summary

The TrenchDB routing layer is built around a regional DHT architecture with XOR-based routing. Nodes maintain compact routing tables composed of peers distributed across distance buckets rather than complete membership lists. New peers are discovered organically through bootstrapping, lookups, and periodic maintenance, eliminating the need for centralized coordinators or broadcast-based discovery.

This architecture provides logarithmic routing performance, automatic topology repair, efficient scaling to millions of nodes, and strict regional data sovereignty while ensuring that every node participates equally in routing and storage.
