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

/// Type of a name operation.
pub enum OperationType {
    Registration,
    Update,
}

/// All data necessary to actually perform a name operation.  This is returned
/// by the validation function, and can then be passed to the execution function
/// if a runtime wants to do its own logic in addition.
pub struct Operation<T: Trait> {
    /// Type of this operation.
    pub operation: OperationType,
    /// The name being operated on.
    pub name: T::Name,
    /// The value it is being set to.
    pub value: T::Value,
    /// The owner it is sent to.
    pub recipient: T::AccountId,
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
            let data = Self::check_assuming_signed(who, name, Some(value), None)?;
            Self::execute(data);
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {

    /// Checks if a name operation is valid, assuming that we already know
    /// it was signed by the given account.  Value and recipient are optional.
    /// If the value is missing, we use the existing value or the default
    /// value if the name does not exist yet.  If the recipient is missing,
    /// we set it to the sender account.
    ///
    /// This function returns either an error if the operation is not valid,
    /// or the data that should be passed to execute later on if the transaction
    /// is valid.
    pub fn check_assuming_signed(sender: T::AccountId, name: T::Name,
                                 value: Option<T::Value>,
                                 recipient: Option<T::AccountId>) -> Result<Operation<T>, &'static str> {
        let (typ, old_value) = match <Names<T>>::get(&name) {
            None => (OperationType::Registration, T::Value::default()),
            Some(data) => {
                ensure!(sender == data.owner, "non-owner name update");
                (OperationType::Update, data.value)
            },
        };

        Ok(Operation::<T> {
            operation: typ,
            name: name,
            value: match value {
                None => old_value,
                Some(new_value) => new_value,
            },
            recipient: match recipient {
                None => sender,
                Some(new_recipient) => new_recipient,
            },
        })
    }

    /// Executes the state change (and fires events) for a given name operation.
    /// This should be called after check_assuming_signed (passing its result),
    /// and when potential other checks have been done as well.
    pub fn execute(op: Operation<T>) {
        let data = NameData::<T> {
            value: op.value,
            owner: op.recipient,
        };

        match op.operation {
            OperationType::Registration => {
                Self::deposit_event(RawEvent::NameRegistered(op.name.clone()));
            },
            OperationType::Update => (),
        }
        Self::deposit_event(RawEvent::NameUpdated(op.name.clone(), data.clone()));

        <Names<T>>::insert(op.name, data);
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
