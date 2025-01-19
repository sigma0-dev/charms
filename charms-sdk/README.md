This is the only crate you need to get started coding a Charms app.

## Usage

Run this command to create a new Charms app:

```sh
charms app new my-app
```

It will create a new directory called `my-app` with a basic Charms app template.

It'll have this in `Cargo.toml`:

```toml
[dependencies]
charms-sdk = { version = "0.3.0" }
```

This is how the entire `src/main.rs` looks like:

```rust
#![no_main]
charms_sdk::main!(my_app::app_contract);
```

The most important function in the app is `app_contract` in `src/lib.rs`:

```rust
use charms_sdk::data::{
    check, App, Data, Transaction, NFT, TOKEN,
};

pub fn app_contract(app: &App, tx: &Transaction, x: &Data, w: &Data) -> bool {
    match app.tag {
        NFT => {
            check!(nft_contract_satisfied(app, tx, x, w))
        }
        TOKEN => {
            check!(token_contract_satisfied(app, tx, x, w))
        }
        _ => todo!(),
    }
    true
}
``` 
