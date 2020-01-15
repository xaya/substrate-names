/// Unit tests for the names module.

use super::*;

use sp_core::H256;
use frame_support::{
    impl_outer_event, impl_outer_origin, parameter_types,
    assert_noop, assert_ok,
    dispatch::DispatchError::BadOrigin,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Test;
parameter_types! {
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

mod names {
    pub use crate::Event;
}
impl_outer_event! {
    pub enum TestEvent for Test {
        names<T>,
    }
}

impl Trait for Test {
    type Name = u64;
    type Value = u64;
    type Event = TestEvent;
}

fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

type System = system::Module<Test>;
type Mod = Module<Test>;

/* ************************************************************************** */

/// Basic tests for the extrinsics themselves.  Most detailed verification
/// is done on the tests for check_assuming_signed and execute, so these just
/// ensure the extrinsics use those methods correctly.
mod extrinsics {
    use super::*;

    #[test]
    fn registration() {
        new_test_ext().execute_with(|| {
            assert_eq!(<Names<Test>>::get(100), None);
            assert_ok!(Mod::update(Origin::signed(10), 100, 42));
            assert_noop!(Mod::update(Origin::ROOT, 200, 30), BadOrigin);
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 42,
                owner: 10,
            }));
            assert_eq!(<Names<Test>>::get(200), None);
        });
    }

    #[test]
    fn update() {
        new_test_ext().execute_with(|| {
            assert_ok!(Mod::update(Origin::signed(10), 100, 42));
            assert_ok!(Mod::update(Origin::signed(10), 100, 50));
            assert_noop!(Mod::update(Origin::signed(20), 100, 666),
                         "non-owner name update");
            assert_noop!(Mod::update(Origin::ROOT, 100, 666), BadOrigin);
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 50,
                owner: 10,
            }));
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
            assert_ok!(Mod::check_assuming_signed(10, 100, None, None), Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 0,
                recipient: 10,
            });
        });
    }

    #[test]
    fn registration_with_values() {
        new_test_ext().execute_with(|| {
            assert_ok!(Mod::check_assuming_signed(10, 100, Some(42), Some(20)), Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 42,
                recipient: 20,
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
                recipient: 10,
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
                recipient: 20,
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
            Mod::execute(Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 42,
                recipient: 10,
            });
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 42,
                owner: 10,
            }));

            Mod::execute(Operation {
                operation: OperationType::Update,
                name: 100,
                value: 50,
                recipient: 20,
            });
            assert_eq!(<Names<Test>>::get(100), Some(NameData::<Test> {
                value: 50,
                owner: 20,
            }));
        });
    }

    #[test]
    fn events() {
        new_test_ext().execute_with(|| {
            Mod::execute(Operation {
                operation: OperationType::Registration,
                name: 100,
                value: 42,
                recipient: 10,
            });
            Mod::execute(Operation {
                operation: OperationType::Update,
                name: 100,
                value: 50,
                recipient: 20,
            });

            assert_eq!(System::events(), vec![
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
            ]);
        });
    }

}
