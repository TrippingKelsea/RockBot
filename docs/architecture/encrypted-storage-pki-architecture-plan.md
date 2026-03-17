# Encrypted Storage and Distributed Vault Architecture Plan

## Purpose

This document formalizes the target architecture for encrypted local storage,
distributed vault authority, PKI-backed roles, and row-level replication.

It is the implementation-oriented companion to
[`encrypted-storage-pki-refactor-proposal.md`](./encrypted-storage-pki-refactor-proposal.md).

## Design Principles

1. Security-first defaults.
2. Explicit role separation.
3. Local at-rest encryption must be node-local.
4. Secret sharing must happen through logical replicated rows, not copied store
   files.
5. Identity, local storage protection, and vault encryption must use separate
   key domains.
6. Gateway and vault-provider are independent roles.
7. Threshold/quorum authority is deferred, but the design must leave room for it.

## Roles

### Gateway

Responsibilities:

- terminate client traffic
- route sessions and agent activity
- coordinate remote execution
- request secret access from vault-providers when needed

Does not inherently imply vault authority.

### Vault-Provider

Responsibilities:

- hold authority to decrypt canonical vault object material
- issue and revoke per-node grants
- enforce vault policy on committed cluster state

May be deployed without gateway responsibilities.

### Client

Responsibilities:

- connect to gateways
- receive node-targeted vault grants
- decrypt only grants addressed to itself

### Admin

Responsibilities:

- bootstrap cluster trust
- authorize node roles
- rotate keys and grants
- perform migrations and recovery procedures

## Key Hierarchy

### Identity Keypair

Used for:

- mTLS identity
- Noise identity
- enrollment and role binding

Stored outside encrypted data, because it is needed to establish trust.

### Local Storage Key

Used for:

- node-local redb encryption

Properties:

- unique per node
- never replicated as a raw secret
- protects local disk state only

### Vault Encryption Keypair

Used for:

- receiving vault grants addressed to the node

Properties:

- distinct from the identity keypair
- bound to identity through PKI certificate extension or signed enrollment
- rotatable independently

### Canonical Vault Object Material

Used for:

- representing the logical protected content of a vault object

Properties:

- encrypted separately for authorized vault-providers
- re-encrypted into per-node grants for consumer nodes

## Trust Bindings

Each node advertises:

- `node_id`
- identity certificate / fingerprint
- vault public key
- certified role set
- status and rotation information

The cluster must be able to prove:

- which identity owns a vault public key
- which identities hold `vault-provider` authority
- which authorities are revoked

## Storage Layers

### Layer 1: Local redb Encryption

All redb-backed state should be encrypted at rest by default with the node's
local storage key.

Applies to:

- sessions
- cron jobs
- routing state
- agents
- vault tables
- PKI index tables
- other persistent redb-backed data

### Layer 2: Vault Object and Grant Encryption

Sensitive vault material is additionally encrypted as logical payloads before
being written into replicated rows.

This ensures replicated data remains meaningful across nodes with different
local storage keys.

## Table Model

### `node_keys`

Stores:

- `node_id`
- identity fingerprint
- vault public key
- role set
- binding certificate or signature
- status
- created / rotated / revoked timestamps

Replication:

- eager

### `vault_objects`

Stores:

- `object_id`
- namespace / tenant / owner
- metadata
- policy reference
- object version
- audit fields

Replication:

- eager

### `vault_provider_grants`

Stores one encrypted copy of canonical vault object material per authorized
vault-provider.

Fields:

- `object_id`
- `provider_node_id`
- ciphertext
- algorithm
- key id
- nonce / IV
- version

Replication:

- eager

### `vault_node_grants`

Stores one encrypted copy of vault object material per authorized consumer node.

Fields:

- `object_id`
- `consumer_node_id`
- ciphertext
- algorithm
- key id
- nonce / IV
- version
- issued-by provider id

Replication:

- eager

### `vault_policies`

Stores:

- namespace rules
- role-based access rules
- provider authority scope
- rotation constraints

Replication:

- eager

### `vault_audit`

Stores:

- grant issuance
- grant revocation
- policy updates
- provider authorization and deauthorization
- key rotation events

Replication:

- eventual or eager, depending on operator requirements

## Secret Distribution Flow

### Grant Issuance

1. A vault-provider receives a request to share or refresh an object for a node.
2. It validates the target node identity, role, and policy against committed
   cluster state.
3. It decrypts canonical object material using its provider grant.
4. It encrypts a node-specific grant to the target node's vault public key.
5. It writes or updates the `vault_node_grants` row.
6. The row replicates through raft.
7. The recipient node stores the row in its own encrypted redb and decrypts it
   locally when needed.

### Grant Revocation

1. A vault-provider revokes the target node's access.
2. The corresponding `vault_node_grants` row is deleted or version-invalidated.
3. The revocation replicates through raft.
4. The target node loses access on the next policy/grant refresh.

### Provider Authorization

1. An admin authorizes a node as `vault-provider`.
2. Canonical object material is encrypted to that provider's vault public key.
3. Provider grant rows replicate.
4. The new provider can now issue node grants based on committed state.

## Replication Model

Raft replicates logical row operations:

- put row
- delete row
- version and audit metadata

It does not replicate raw redb file bytes or node-local ciphertext.

This means:

- local storage encryption remains node-local
- vault confidentiality is preserved by grant-layer encryption
- row-level synchronization remains compatible with the existing raft direction

## PKI Responsibilities

PKI remains responsible for:

- issuing identity certificates
- carrying role assertions
- binding node vault keys to identity
- revoking identities and authorities
- enabling secure enrollment

PKI should evolve so protected PKI state is stored in encrypted redb-backed
storage instead of relying exclusively on plaintext file-based metadata.

The file backend can remain as an export or compatibility path, but should not
remain the long-term default security model.

## Configuration Direction

Illustrative configuration shape:

```toml
[security.storage]
enabled = true
mode = "encrypted_by_default"

[security.roles]
gateway = true
vault_provider = false

[security.vault]
grant_model = "per_node"
provider_mode = "cluster"

[security.pki]
bind_vault_keys_to_identity = true
```

The exact schema may change, but the role and trust intent should be explicit.

## Tooling Requirements

This refactor should ship with operator-facing tooling, not just internal code.

Required command areas:

- `rockbot init cluster`
  - bootstrap CA / PKI
  - create first vault-provider
  - initialize secure storage defaults
- `rockbot node enroll`
  - create identity keypair
  - create vault keypair
  - register node and requested roles
  - initialize local storage key
- `rockbot vault-provider authorize`
  - grant or revoke provider authority
- `rockbot vault grant`
  - grant node access to an object or namespace
- `rockbot vault rotate`
  - rotate keys, grants, or object protection
- `rockbot migrate storage`
  - migrate plaintext redb and legacy PKI/vault state
- `rockbot doctor security`
  - verify encrypted storage
  - detect plaintext legacy paths
  - verify role consistency
  - verify grant and key health

## Migration Plan

### Phase 1: Authenticated Encrypted redb

- replace the current confidentiality-only backend
- introduce encrypted-by-default local storage
- migrate plaintext stores

### Phase 2: Node Vault Keys and Role Registry

- generate node vault keypairs
- bind them to PKI identity
- add replicated `node_keys` and role metadata

### Phase 3: Logical Vault Objects and Grants

- add `vault_objects`
- add `vault_provider_grants`
- add `vault_node_grants`
- implement per-node secret distribution

### Phase 4: Multi-Provider Authority

- authorize multiple `vault-provider` nodes
- enforce provider actions against committed raft state
- support provider-only nodes that are not gateways

### Phase 5: PKI Storage Hardening

- move PKI index/state into encrypted storage
- reduce reliance on plaintext file-backed key state
- improve rotation and recovery flows

### Phase 6: Threshold / Quorum Exploration

- design split authority model
- evaluate threshold issuance for high-risk objects
- keep disabled by default until operationally mature

## Deferred Items

These are intentionally out of scope for the first implementation wave:

- mandatory threshold cryptography
- full HSM/PKCS#11 integration as the default path
- transparent live rekeying of every historical object without migration tooling

## Success Criteria

The refactor is successful when:

- local redb stores are encrypted by default
- gateway and vault-provider roles are independently deployable
- per-node vault grants replicate and decrypt correctly
- multiple vault-provider nodes can issue grants safely
- deployment tooling makes secure setup routine
- the design remains compatible with a later threshold/quorum authority model
