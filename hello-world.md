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

You will need to have `bitcoin-cli` aliased as `b`:

```sh
alias b=bitcoin-cli
```

You will also need to have `jq` installed:

```sh
brew install jq
```

## Installation

If you're using the v0.1.0 release, you can download the `charms-spell-checker` binary
from: https://github.com/sigma0-dev/charms/releases/tag/v0.1.0 and place it at
`./target/elf-compilation/riscv32im-succinct-zkvm-elf/release/charms-spell-checker`. You can verify that it's the
correct binary by running:

````sh
sha1sum ./target/elf-compilation/riscv32im-succinct-zkvm-elf/release/charms-spell-checker
````

It should print out this:

```
ead39847d01fd2c88af9a152366e78bf1399ad61  ./target/elf-compilation/riscv32im-succinct-zkvm-elf/release/charms-spell-checker
```

It is required to be able to install the `charms` CLI binary.

Install Charms:

```sh
cargo test && cargo install --path .
```

Check that the installation was successful:

```sh
pushd ./examples/toad-token
cargo prove build
popd

charms app vk ./examples/toad-token/elf/riscv32im-succinct-zkvm-elf
```

This will print out the verification key for the Toad Token app, that looks something like:

```
8e877d70518a5b28f5221e70bd7ff7692a603f3a26d7076a5253e21c304a354f
```

## Walkthrough

```sh
recipient="$(b getnewaddress)"

rawtxhex=$(b createrawtransaction '''[]''' '''[{ "'$recipient'": 0.00010000 }]''')

# or something like this if you want to spend an existing Charm
# rawtxhex=$(b createrawtransaction '''[ { "txid": "dafd94568e0d8fb0e72c9bb84e54b227c9cad28168611fe3d37f06276125e247", "vout": 0 } ]''' '''[{ "'$recipient'": 0.00010000 }]''')

fee_rate=2

draft_tx_hex=$(b -named fundrawtransaction $rawtxhex changePosition=1 fee_rate=$fee_rate | jq -r '.hex')

# now choose a funding output with a few thousand sats (50000 should be more than enough)
b listunspent

# MUST NOT be one of those being spent by $draft_tx_hex (printed out by this)
b decoderawtransaction $(echo $draft_tx_hex) | jq -r '.vin[] | "\(.txid):\(.vout)"'

funding_utxo_id=acbef6b2f3808ad4fe36fff4d70ba1d0ccc05ce254d8096a8591de76683af8d0:0
funding_utxo_value=10000
# value in sats

change_address=$(b getrawchangeaddress)

b decoderawtransaction $draft_tx_hex

# now get the hex representation of $draft_tx_hex's input transactions

prev_txs=$(b decoderawtransaction $draft_tx_hex | jq -r '.vin[].txid' | sort | uniq | xargs -I {} bitcoin-cli getrawtransaction {} | paste -sd, -)

spell_source=./examples/toad-token/spells/mint-token.yml
toad_app_bin=./examples/toad-token/elf/riscv32im-succinct-zkvm-elf

RUST_LOG=info charms spell prove --spell=$spell_source --tx=$draft_tx_hex --prev-txs=$prev_txs --app-bins=$toad_app_bin --funding-utxo-id=$funding_utxo_id --funding-utxo-value=$funding_utxo_value --change-address=$change_address --fee-rate=$fee_rate

# sign the resulting transactions
# copy the output from the previous command into spell_prove_result:

spell_prove_result='["0200000001d0...000","020000000001041...000"]'

signed_commit_tx=$(b signrawtransactionwithwallet $(echo $spell_prove_result | jq -r '.[0]') | jq -r '.hex')

signed_spell_tx=$(b signrawtransactionwithwallet $(echo $spell_prove_result | jq -r '.[1]') $(b decoderawtransaction $signed_commit_tx | jq -c '[{txid: .txid, vout: .vout[0].n, scriptPubKey: .vout[0].scriptPubKey.hex, amount: .vout[0].value}]') | jq -r '.hex')

b submitpackage '["'$signed_commit_tx'","'$signed_spell_tx'"]'
```
