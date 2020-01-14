#![cfg_attr(not(feature = "std"), no_std)]

/// A pallet that defines a system to register and update names
/// in a Substrate chain.  This provides (roughly) the functionality
/// of the Namecoin blockchain.
///
/// The core concept is that of a "name".  This is some identifier (the exact
/// type can be configured through the module's Trait), e.g. a human-readable
/// name as string.  Each name is unique, and has an associated value and owner.
/// Everyone can read the database of names, but only the owner can make
/// changes to it.  This typically means changing the value to publish some
/// data with the name, but the owner can also transfer names to a different
/// owner.
///
/// Names are given out on a first come, first serve basis.  Each name that
/// is not yet registered (and valid for the system) can be registered by
/// any account (which may incur a fee for registration, and then maybe also
/// for updates to the name).  Once registered, the name is owned by the
/// account that first registered it.
///
/// After a certain number of blocks, names may expire and become usable again.
/// By updating a name before the expiration, the current owner can keep
/// ownership.
///
/// The names module defines basic extrinsics to perform name operations
/// (register / update / transfer names) and events corresponding to changes
/// in the name database.  But if custom logic needs to be applied in addition
/// by the runtime, it may use the exposed functions check_assuming_signed
/// and execute directly.

use frame_support::{
    decl_module, decl_storage, decl_event, ensure,
    dispatch::DispatchResult, dispatch::fmt::Debug,
};
use codec::{Decode, Encode, FullCodec};
use system::ensure_signed;

/// The pallet's configuration trait.
pub trait Trait: system::Trait {
    /// Type for names.
    type Name: Clone + Debug + Default + Eq + FullCodec;
    /// Type for name values.
    type Value: Clone + Debug + Default + Eq + FullCodec;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// All data stored with a name in the database.
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Decode, Encode, Eq, PartialEq)]
pub struct NameData<T: Trait> {
    /// The name's associated value.
    pub value: T::Value,
    /// The name's current owner.
    pub owner: T::AccountId,
}

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule {
        Names: map T::Name => Option<NameData<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Tries to update a name with a given value.  If the name does not
        /// exist yet, it will be created.  If the name exists, then only the
        /// current owner can update it.
        pub fn update(origin, name: T::Name, value: T::Value) -> DispatchResult {
            let who = ensure_signed(origin)?;

            /* If the name exists, it can only be updated by the current
               owner account.  */
            let created = match <Names<T>>::get(&name) {
                None => true,
                Some(data) => {
                    ensure!(who == data.owner, "non-owner name update");
                    false
                },
            };

            /* All is valid, so we can update the database and fire events.  */

            let data = NameData::<T> {
                value: value,
                owner: who,
            };

            if created {
                Self::deposit_event(RawEvent::NameRegistered(name.clone()));
            }
            Self::deposit_event(RawEvent::NameUpdated(name.clone(), data.clone()));

            <Names<T>>::insert(name, data);

            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T> where Name = <T as Trait>::Name, NameData = NameData<T> {
        /// Event when a name is newly created.
        NameRegistered(Name),
        /// Event when a name is updated (or created).
        NameUpdated(Name, NameData),
    }
);

/// tests for this pallet
#[cfg(test)]
mod tests {
    use super::*;

    use sp_core::H256;
    use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
    };

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    // For testing the pallet, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq)]
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
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
    }
    impl Trait for Test {
        type Event = ();
    }
    type TemplateModule = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> sp_io::TestExternalities {
        system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
    }

    #[test]
    fn it_works_for_default_value() {
        new_test_ext().execute_with(|| {
            // Just a dummy test for the dummy funtion `do_something`
            // calling the `do_something` function with a value 42
            assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
            // asserting that the stored value is equal to what we stored
            assert_eq!(TemplateModule::something(), Some(42));
        });
    }
}
