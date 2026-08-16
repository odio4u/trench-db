# Node Authentication and Network Trust

GOAL: Establish a trust model where TLS protects the transport and node membership is verified by network-signed credentials. This removes the need for a static CA bundle pinned on every node and instead relies on a bootstrap trust anchor plus locally cached trusted peer credentials.

```Mermaid
architecture-beta
    %% group api(cloud)[API]

    %% service db(database)[Database] in api
    %% service disk1(disk)[Storage] in api
    %% service disk2(disk)[Storage] in api
    %% service server(server)[Server] in api

    %% db:L -- R:server
    %% disk1:T -- B:server
    %% disk2:T -- B:db

    group Network(cloud)[Network]

    service Node1(server)[Node1] in Network
    service Node2(server)[Node2] in Network
    service Node3(server)[Node3] in Network
    service Node4(server)[Node4] in Network

    Node1:L -- R:Node2
    Node1:R -- L:Node3
    Node2:L -- L:Node4
    Node3:R -- R:Node4
```



Network topology for a distributed system with four nodes (Node1, Node2, Node3, and Node4) is illustrated above. Each node can communicate with its neighbouring nodes, ensuring redundancy and fault tolerance.

## Node Identity

Node identity is based on stable node identifiers plus signed credentials.

{
    ID: Unique node identifier (UUID)
    PublicKey: The node's long-term public key
    Credential: A signed proof binding the node ID to the public key
    Issuer: The trusted party or parties that signed the credential
    Fingerprint: A hash of the credential or public key for fast verification
}

## Trust Model

- TLS is used for encryption and transport protection.
- Node membership is granted by network-signed credentials, not by a pinned CA bundle.
- Each node maintains a local trust cache of validated peer credentials.
- A bootstrap trust anchor bootstraps initial membership, then trust is propagated through signed credentials.
- The bootstrap node is a seed for trust, not a permanent runtime authority.

## Bootstrap Admission

The network starts from one or more bootstrap nodes.

- Bootstrap nodes provide the initial trust anchor and may self-issue credentials.
- A new node begins with bootstrap peer information in its configuration.
- The new node generates a long-term key pair and constructs a node credential request.
- A trusted bootstrap peer verifies the request and issues a signed credential.
- The joining node stores its signed credential and uses it for future peer authentication.

After bootstrapping, trusted peers can accept the new node using the locally cached credential and can also be used to admit additional nodes if the admission policy allows.

Bootstrap node is only essential for the initial setup of the network. Once the network is established, nodes can communicate and authenticate with each other using their unique identifiers and signatures. The bootstrap node can be removed from the network after the initial setup, as long as there are enough trusted nodes to maintain the integrity of the network.


Bootstrap to Node1:

The new node begins with a bootstrap configuration containing known bootstrap peers.

The config file should contain:
- Node ID: ID for the bootstrap node
- Address: The network address of the bootstrap peer

During admission:
- The joining node uses TLS to connect to a bootstrap peer.
- The joining node presents its generated public key and node identifier.
- The bootstrap peer verifies the joining node and issues a signed node credential.
- The joining node stores its credential and uses it for future peer authentication.

## Node-to-node Communication

After initial admission, nodes communicate using their signed credentials. The process is:

1. **Handshake**: Node A connects to Node B over TLS and presents its signed node credential.
2. **Verification**: Node B validates the credential signature chain, verifies the issuer, and checks whether Node A is trusted locally.
3. **Secure Communication**: If verification succeeds, both nodes complete the TLS session and exchange application traffic.

Local trust caching means nodes do not need to contact the bootstrap authority for every connection. This reduces latency and improves performance while preserving secure network membership verification.

## Design Principles

- Separate transport-layer encryption from membership validation.
- Use stable node IDs and signed credentials rather than IP-based identity.
- Cache trusted peer credentials locally to avoid repeated central validation.
- Explicitly define admission and revocation rules to prevent stale or compromised trust.
- Keep bootstrap trust anchor use limited to initial admission, not every connection.

## Practical Notes

- Avoid relying on IP address as the principal identity field.
- Prefer credential refresh or revocation lists to handle compromised keys.
- Trust should move local after initial validation to reduce latency.
- The bootstrap anchor is a seed, not a global CA pin replacement.