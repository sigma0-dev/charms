---
CHIP: 420
Title: Token Metadata
Status: Draft
Authors:
  - Ivan Mikushin (@imikushin)
Created: 2025-02-25
---

# CHIP-420. Token Metadata

This proposal borrows ideas from Cardano CIP-0068 Datum Metadata Standard [1] and Solana Metaplex Token Metadata [2].

A fungible token value in Charms is represented by a single unsigned integer. This is great as a compact representation
of the token amount. A fungible token, as an asset, is fully defined by its App structure:

```
t/{app_identity}/{app_vk}
```

If we replace the tag character `t` with `n`, we will have a full identifier for the fungible token's reference NFT.

## Proposal

We propose the following structure for reference NFT data:

```yaml
# (optional) 
# Asset name.
name: ?<string>

# (optional) 
# Description of the fungible token.
description: ?<string>

# (optional) 
# Ticker symbol.
ticker: ?<string>

# (optional)
# Website URL.
# The URI scheme must be one of `https` (HTTP), `ipfs` (IPFS), `ar` (Arweave) or `data` (on-chain).
# Data URLs (on-chain data) must comply to [RFC2397](https://www.rfc-editor.org/rfc/rfc2397).
url: ?<URI>

# (optional) 
# A valid Uniform Resource Identifier (URI).
# The URI scheme must be one of `https` (HTTP), `ipfs` (IPFS), `ar` (Arweave) or `data` (on-chain).
# Data URLs (on-chain data) must comply to [RFC2397](https://www.rfc-editor.org/rfc/rfc2397).
# Should point to a resource with media type `image/png`, `image/jpeg` or `image/svg+xml`.
image: ?<URI>

# (optional)
# SHA-256 hash of the remote resource that `image` is pointing to.
# When not present, the resource is assumed to be mutable (which is 100% okay for a home page of a website),
# unless `image` is a `data` URI.
image_hash: ?<hex_string>

# (optional) 
# Number of digits after decimal point in the smallest denomination of the token.
# A token amount in Charms is always a natural number quantity of its smallest denomination. 
# For example, for BTC `decimals == 8`: the smallest denomination, satoshi (sat), is 10^(-8) * 1 BTC.
# So, `0.01234567` BTC is represented onchain as `1234567` sats.
# The default is `decimals == 0`: the smallest denomination is 1 token.
decimals: ?<u8>

# (optional)
# UTXO with upstream data of this NFT.
# A useful optimization: if we want to (cheaply) transfer the NFT without copying its full data, 
# we can use this field to refer to already existing UTXO where this NFT can be found.
ref: ?<UtxoId>

# ... Additional fields are allowed.
```

The above is, of course, not the complete set of fields for NFTs, but rather an extensible recommended
minimum (recommended, but not required: all fields are optional).

### References

[1] Cardano CIP-0068 Datum Metadata Standard. https://github.com/cardano-foundation/CIPs/tree/master/CIP-0068

[2] Solana Metaplex Token Metadata. https://developers.metaplex.com/token-metadata
