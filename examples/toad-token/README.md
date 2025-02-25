This is a [Charms](https://charms.dev) app.

It is a simple fungible token managed by a reference NFT. The NFT has a state that specifies the remaining total supply
of the tokens available to mint. If you control the NFT, you can mint new tokens.

Build with:

```sh
charms app build
```

The resulting RISC-V binary will show up at `./target/charms-app`.

Get the verification key for the app with:

```sh
charms app vk
```

Test the app with a simple NFT mint example:

```sh
export app_vk=$(charms app vk)

# set to a UTXO you're spending (you can see what you have by running `b listunspent`)
export in_utxo_0="a2889190343435c86cd1c2b70e58efed0d101437a753e154dff1879008898cd2:2"

export app_id=$(echo -n "${in_utxo_0}" | sha256sum | cut -d' ' -f1)
export addr_0="tb1p3w06fgh64axkj3uphn4t258ehweccm367vkdhkvz8qzdagjctm8qaw2xyv"

cat ./spells/mint-nft.yaml | envsubst | charms app run
```
