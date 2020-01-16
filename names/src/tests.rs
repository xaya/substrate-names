/// Unit tests for the names module.

use super::*;

use sp_core::H256;
use frame_support::{
    impl_outer_event, impl_outer_origin, parameter_types,
    assert_noop, assert_ok,
    dispatch::DispatchError,
    traits::{Imbalance, LockableCurrency, ReservableCurrency, WithdrawReasons},
    weights::Weight,
};
use system::{EventRecord, Phase};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};
use crate::{Module, Trait};

impl_outer_origin! {
    pub enum Origin for Test {}
}

/// Account ID that receives name fees.
const FEE_RECEIVER: u64 = 12345;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const ExistentialDeposit: u128 = 1000;
    pub const TransferFee: u128 = 0;
    pub const CreationFee: u128 = 0;
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl system::Trait for Test {
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
}
impl balances::Trait for Test {
    type Balance = u128;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type Event = TestEvent;
    type DustRemoval = ();
    type TransferPayment = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = ();
    type CreationFee = ();
}

mod names {
    pub use crate::Event;
}
impl_outer_event! {
    pub enum TestEvent for Test {
        balances<T>,
        names<T>,
    }
}

impl Trait for Test {

    type Name = u64;
    type Value = u64;

    type Currency = Balances;
    type Event = TestEvent;

    fn get_name_fee(op: &Operation<Self>) -> u128 {
        match op.operation {
            OperationType::Registration => 100,
            OperationType::Update => 0,
        }
    }

    fn deposit_fee(neg: <Self::Currency as Currency<u64>>::NegativeImbalance) {
        let value = neg.peek();
        let pos = Balances::deposit_creating(&FEE_RECEIVER, value);
        let result = pos.offset(neg).ok().expect("fee balances offset failed");
        result.drop_zero().ok().expect("fee balances mismatch");
    }

}

fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

type System = system::Module<Test>;
type Balances = balances::Module<Test>;
type Mod = Module<Test>;

fn add_balance(account: u64, value: u128) {
    let _ = Balances::deposit_creating(&account, value);
}

/// Asserts that the current total balance of the given account matches
/// the expected value.  By having this as a convenience function, we can
/// easily pass in the account as literal integer (while calling
/// Balances::total_balance directly would mean we need a variable that we
/// can take a reference of).
fn expect_balance(account: u64, expected: u128) {
    assert_eq!(Balances::total_balance(&account), expected);
}

/* ************************************************************************** */

/// Basic tests for the extrinsics themselves.  Most detailed verification
/// is done on the tests for check_assuming_signed and execute, so these just
/// ensure the extrinsics use those methods correctly.
mod extrinsics {
    use super::*;

    #[test]
    fn registration() {
        new_test_ext().execute_with(|| {
            add_balance(FEE_RECEIVER, 1000);
            add_balance(10, 5000);
            assert_eq!(<Names<Test>>::get(100), None);
            assert_ok!(Mod::update(Origin::signed(10), 100, 42));
            assert_noop!(Mod::update(Origin::ROOT, 200, 30),
                         DispatchError::BadOrigin);
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 42,
                owner: 10,
            }));
            assert_eq!(<Names<Test>>::get(200), None);
            expect_balance(FEE_RECEIVER, 1100);
            expect_balance(10, 4900);
        });
    }

    #[test]
    fn update() {
        new_test_ext().execute_with(|| {
            add_balance(FEE_RECEIVER, 1000);
            add_balance(10, 5000);
            assert_ok!(Mod::update(Origin::signed(10), 100, 42));
            assert_ok!(Mod::update(Origin::signed(10), 100, 50));
            assert_noop!(Mod::update(Origin::signed(20), 100, 666),
                         "non-owner name update");
            assert_noop!(Mod::update(Origin::ROOT, 100, 666),
                         DispatchError::BadOrigin);
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 50,
                owner: 10,
            }));
            expect_balance(FEE_RECEIVER, 1100);
            expect_balance(10, 4900);
        });
    }

    #[test]
    fn transfer() {
        new_test_ext().execute_with(|| {
            add_balance(FEE_RECEIVER, 1000);
            add_balance(10, 5000);
            assert_noop!(Mod::transfer(Origin::ROOT, 100, 42),
                         DispatchError::BadOrigin);
            assert_ok!(Mod::transfer(Origin::signed(10), 100, 20));
            assert_noop!(Mod::transfer(Origin::signed(10), 100, 30),
                         "non-owner name update");
            assert_ok!(Mod::update(Origin::signed(20), 100, 99));
            assert_ok!(Mod::transfer(Origin::signed(20), 100, 40));
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 99,
                owner: 40,
            }));
            expect_balance(FEE_RECEIVER, 1100);
            expect_balance(10, 4900);
        });
    }

}

/* ************************************************************************** */

/// Unit tests for the check_assuming_signed function.
mod check_function {
    use super::*;

    #[test]
    fn registration_defaults() {
        new_test_ext().execute_with(|| {
            add_balance(10, 5000);
            assert_ok!(Mod::check_assuming_signed(10, 100, None, None), Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 0,
                sender: 10,
                recipient: 10,
                fee: 100,
            });
        });
    }

    #[test]
    fn registration_with_values() {
        new_test_ext().execute_with(|| {
            add_balance(10, 5000);
            assert_ok!(Mod::check_assuming_signed(10, 100, Some(42), Some(20)), Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 42,
                sender: 10,
                recipient: 20,
                fee: 100,
            });
        });
    }

    #[test]
    fn update_nonowner() {
        new_test_ext().execute_with(|| {
            <Names<Test>>::insert(100, NameData {
                value: 42,
                owner: 20,
            });
            assert_noop!(Mod::check_assuming_signed(10, 100, None, None), "non-owner name update");
        });
    }

    #[test]
    fn update_defaults() {
        new_test_ext().execute_with(|| {
            <Names<Test>>::insert(100, NameData {
                value: 42,
                owner: 10,
            });
            assert_ok!(Mod::check_assuming_signed(10, 100, None, None), Operation {
                operation: OperationType::Update,
                name: 100,
                value: 42,
                sender: 10,
                recipient: 10,
                fee: 0,
            });
        });
    }

    #[test]
    fn update_with_values() {
        new_test_ext().execute_with(|| {
            <Names<Test>>::insert(100, NameData {
                value: 42,
                owner: 10,
            });
            assert_ok!(Mod::check_assuming_signed(10, 100, Some(50), Some(20)), Operation {
                operation: OperationType::Update,
                name: 100,
                value: 50,
                sender: 10,
                recipient: 20,
                fee: 0,
            });
        });
    }

    #[test]
    fn balance_checks() {
        new_test_ext().execute_with(|| {
            assert_noop!(Mod::check_assuming_signed(10, 100, None, None),
                         "insufficient balance for name fee");

            let locked_account = 20;
            add_balance(locked_account, 5000);
            <Balances as LockableCurrency<u64>>::set_lock(
                [1, 2, 3, 4, 5, 6, 7, 8], &locked_account,
                4901, 100, WithdrawReasons::all());
            assert_noop!(Mod::check_assuming_signed(locked_account, 100, None, None),
                         "cannot withdraw name fee from sender");

            let reserved_account = 30;
            add_balance(reserved_account, 5000);
            assert_ok!(<Balances as ReservableCurrency<u64>>::reserve(
                            &reserved_account, 4901));
            assert_noop!(Mod::check_assuming_signed(reserved_account, 100, None, None),
                         "insufficient balance for name fee");

            let ok_account = 40;
            add_balance(ok_account, 2100);
            <Balances as LockableCurrency<u64>>::set_lock(
                [1, 2, 3, 4, 5, 6, 7, 8], &ok_account,
                1000, 100, WithdrawReasons::all());
            assert_ok!(<Balances as ReservableCurrency<u64>>::reserve(
                            &ok_account, 1000));
            assert_ok!(Mod::check_assuming_signed(ok_account, 100, Some(50), Some(20)), Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 50,
                sender: ok_account,
                recipient: 20,
                fee: 100,
            });
        });
    }

}

/* ************************************************************************** */

/// Unit tests for the execute function.
mod execute_function {
    use super::*;

    #[test]
    fn updates_storage() {
        new_test_ext().execute_with(|| {
            assert_ok!(Mod::execute(Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 42,
                sender: 10,
                recipient: 10,
                fee: 0,
            }));
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 42,
                owner: 10,
            }));

            assert_ok!(Mod::execute(Operation {
                operation: OperationType::Update,
                name: 100,
                value: 50,
                sender: 10,
                recipient: 20,
                fee: 0,
            }));
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 50,
                owner: 20,
            }));
        });
    }

    #[test]
    fn fee_handling() {
        new_test_ext().execute_with(|| {
            add_balance(FEE_RECEIVER, 1000);
            add_balance(10, 5000);
            assert_ok!(Mod::execute(Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 50,
                sender: 10,
                recipient: 10,
                fee: 50,
            }));
            expect_balance(FEE_RECEIVER, 1050);
            expect_balance(10, 4950);
            assert_eq!(Balances::total_issuance(), 6000);

            /* Verify that we get a noop if the withdrawal fails.  */
            assert_noop!(Mod::execute(Operation {
                operation: OperationType::Update,
                name: 100,
                value: 60,
                sender: 10,
                recipient: 20,
                fee: 5000,
            }), DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance"),
            });

            /* Process a situation where the account gets killed due
               to falling below the existence minimum.  This will then
               kill the account, effectively burning the remaining balance.  */
            assert_ok!(Mod::execute(Operation {
                operation: OperationType::Update,
                name: 100,
                value: 70,
                sender: 10,
                recipient: 10,
                fee: 4000,
            }));
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 70,
                owner: 10,
            }));
            expect_balance(FEE_RECEIVER, 5050);
            expect_balance(10, 0);
            assert_eq!(Balances::total_issuance(), 5050);
        });
    }

    #[test]
    fn events() {
        new_test_ext().execute_with(|| {
            add_balance(FEE_RECEIVER, 1000);
            add_balance(10, 5000);
            let balance_events = System::events();

            assert_ok!(Mod::execute(Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 42,
                sender: 10,
                recipient: 10,
                fee: 0,
            }));
            assert_ok!(Mod::execute(Operation {
                operation: OperationType::Update,
                name: 100,
                value: 50,
                sender: 10,
                recipient: 20,
                fee: 0,
            }));

            let name_events = vec![
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: TestEvent::names(RawEvent::NameRegistered(100)),
                    topics: vec![],
                },
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: TestEvent::names(RawEvent::NameUpdated(100, NameData {
                        value: 42,
                        owner: 10,
                    })),
                    topics: vec![],
                },
                EventRecord {
                    phase: Phase::ApplyExtrinsic(0),
                    event: TestEvent::names(RawEvent::NameUpdated(100, NameData {
                        value: 50,
                        owner: 20,
                    })),
                    topics: vec![],
                },
            ];
            assert_eq!(System::events(),
                       [&balance_events[..], &name_events[..]].concat());
        });
    }

}
