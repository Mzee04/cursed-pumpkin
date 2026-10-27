//! Test-only helper: hands out a public fungible "bond coin" so engine tests can stake bonds without TARI.
use tari_template_lib::prelude::*;

#[template]
mod test_coin {
    use super::*;

    pub struct TestCoin {
        coins: Vault,
    }

    impl TestCoin {
        pub fn new(supply: Amount) -> Component<Self> {
            let coins = ResourceBuilder::public_fungible()
                .with_token_symbol("BOND")
                .initial_supply(supply);
            Component::new(Self { coins: Vault::from_bucket(coins) })
                .with_access_rules(AccessRules::allow_all())
                .create()
        }

        pub fn resource(&self) -> ResourceAddress {
            self.coins.resource_address()
        }

        pub fn take(&mut self, amount: Amount) -> Bucket {
            self.coins.withdraw(amount)
        }
    }
}
