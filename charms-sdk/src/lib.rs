pub use charms_data as data;
pub use sp1_zkvm;

#[macro_export]
macro_rules! main {
    ($path:path) => {
        fn main() {
            use charms_sdk::data::{App, Data, Transaction};
            let (app, tx, x, w): (App, Transaction, Data, Data) = charms_sdk::sp1_zkvm::io::read();
            assert!($path(&app, &tx, &x, &w));
            charms_sdk::sp1_zkvm::io::commit(&(&app, &tx, &x));
        }
        charms_sdk::sp1_zkvm::entrypoint!(main);
    };
}
