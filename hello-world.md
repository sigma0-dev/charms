## Pre-requisites

Bitcoin Core v22.0 or later is required:

```sh
brew install bitcoin
```

This walkthrough assumes a bitcoin node running with the following configuration (`bitcoin.conf`):

```
server=1
testnet4=1
txindex=1
addresstype=bech32m
changetype=bech32m
```

On macOS, `bitcoin.conf` is usually located at `~/Library/Application Support/Bitcoin/bitcoin.conf`.

Alias `bitcoin-cli` as `b` (it's annoying to type `bitcoin-cli` all the time):

```sh
alias b=bitcoin-cli
```

Make sure you have a wallet loaded:

```sh
b createwallet testwallet  # create a wallet (you might already have one)
b loadwallet testwallet    # load the wallet (bitcoind doesn't do it automatically when it starts)
```

Get some test BTC:

```sh
b getnewaddress # prints out a new address associated with your wallet
```

Visit https://mempool.space/testnet4/faucet and get some test BTC to the address you just created. Get at least 50000
sats (0.0005 (test) BTC). Also, get more than one UTXO, so either tap the faucet more than once or send some sats within
your wallet to get some small UTXOs and at least one larger one (>= 10000 sats).

You will need to have `jq` installed (bitcoin-cli output is mostly JSON):

```sh
brew install jq
```

## Installation

Install Charms CLI:

```sh
cargo install charms
```

## Create an app

Run this **outside** the `charms` repo:

```sh
charms app new my-token
cd ./my-token
charms app vk
```

This will print out the verification key for the Toad Token app, that looks something like:

```
8e877d70518a5b28f5221e70bd7ff7692a603f3a26d7076a5253e21c304a354f
```

Test the app for a spell with a simple NFT mint example:

```sh
export app_vk=$(charms app vk)

# set to a UTXO you're spending to mint the NFT (you can see what you have by `b listunspent`)
export in_utxo_0="dc78b09d767c8565c4a58a95e7ad5ee22b28fc1685535056a395dc94929cdd5f:1"

export app_id=$(sha256 -s "${in_utxo_0}")
export addr_0=$(b getnewaddress)

cat ./spells/mint-nft.yaml | envsubst | charms app run
```

If all is well, you should see that the app contract for minting an NFT has been satisfied.

To continue playing with the other spells, keep the same `app_id` value: you create the `app_id` value for a newly
minted NFT, and then keep using it for the lifetime of the NFT and any associated fungible tokens (if the app supports
them).

## Using an app

We've just tested the app with an NFT-minting spell. Let's use it on Bitcoin `testnet4`.

```sh
app_bins=$(charms app build)
cat ./spells/mint-nft.yaml | envsubst | charms wallet cast --app-bins=${app_bins} --funding-utxo-id=${funding_utxo_id}
```

This will create and sign (but not yet submit to the network) two Bitcoin transactions: commit tx and execute tx. The
commit transaction creates an output (committing to a spell and its proof) which is spent by the execute transaction.
The execute transaction is the one that creates the NFT (but it can't exist without the commit tx).

Note: currently, `charms wallet cast` takes a pretty long time (about 27 minutes on MBP M2 64GB) and requires Docker to
run. We're working on improving this.

You submit both transaction to the network as a package, which looks like the following command:

```sh
b submitpackage '["020000000001015f...57505efa00000000", "020000000001025f...e14c656300000000"]'
```
