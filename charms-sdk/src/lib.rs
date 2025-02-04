pub use charms_data as data;
pub use sp1_zkvm;

#[macro_export]
macro_rules! main {
    ($path:path) => {
        fn main() {
            use charms_sdk::data::{is_simple_transfer, util, App, Data, Transaction};

            fn read_input() -> (App, Transaction, Data, Data) {
                let buf = charms_sdk::sp1_zkvm::io::read_vec();
                util::read(&buf[..])
                    .expect("should deserialize (app, tx, x, w): (App, Transaction, Data, Data)")
            }

            fn commit(app: App, tx: Transaction, x: Data) {
                let buf = util::write(&(app, tx, x))
                    .expect("should serialize (app, tx, x): (App, Transaction, Data)");
                charms_sdk::sp1_zkvm::io::commit_slice(&buf[..]);
            }

            let (app, tx, x, w): (App, Transaction, Data, Data) = read_input();
            assert!(is_simple_transfer(&app, &tx) || $path(&app, &tx, &x, &w));
            commit(app, tx, x);
        }

        charms_sdk::sp1_zkvm::entrypoint!(main);
    };
}
