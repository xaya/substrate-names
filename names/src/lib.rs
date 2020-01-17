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
    traits::{Currency, ExistenceRequirement, WithdrawReason, WithdrawReasons},
};
use codec::{Decode, Encode, FullCodec};
use system::ensure_signed;
use sp_runtime::traits::CheckedSub;
use core::cmp::max;

/// The pallet's configuration trait.
pub trait Trait: system::Trait {

    /// Type for names.
    type Name: Clone + Debug + Default + Eq + FullCodec;
    /// Type for name values.
    type Value: Clone + Debug + Default + Eq + FullCodec;

    /// Type for currency operations (in order to pay for names).
    type Currency: Currency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Computes and returns the currency fee the sender has to pay for
    /// a certain operation.  If None is returned, it means that the
    /// operation is invalid (e.g. the name is too short).
    fn get_name_fee(op: &Operation<Self>)
        -> Option<<Self::Currency as Currency<Self::AccountId>>::Balance>;

    /// For a given name operation, compute the number of blocks before the
    /// name will expire again.  If None is returned, then the name will
    /// never expire.
    fn get_expiration(op: &Operation<Self>) -> Option<Self::BlockNumber>;

    /// "Takes ownership" of the fee paid for a name registration.  This
    /// function can just do nothing to effectively burn the fee, it may
    /// deposit it to a developer account, or it may give it out to miners.
    fn deposit_fee(value: <Self::Currency as Currency<Self::AccountId>>::NegativeImbalance);

}

/// All data stored with a name in the database.
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Decode, Encode, Eq, PartialEq)]
pub struct NameData<T: Trait> {
    /// The name's associated value.
    pub value: T::Value,
    /// The name's current owner.
    pub owner: T::AccountId,
    /// The block number when the name expires or None if it does not expire.
    /// While we also have an explicit index mapping block numbers to names
    /// that expire, the value here is stored a) for informative purposes
    /// (so one can query when a certain name expires), and b) so that it
    /// can overrule the expiration index:  If a name is updated, then we
    /// are not removing it from the old expiration index; instead, we will
    /// simply not expire names when processing a the expiration index if their
    /// value here does not match the one from the index.
    pub expiration: Option<T::BlockNumber>,
}

/// Type of a name operation.
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Eq, PartialEq)]
pub enum OperationType {
    Registration,
    Update,
}

/// All data necessary to actually perform a name operation.  This is returned
/// by the validation function, and can then be passed to the execution function
/// if a runtime wants to do its own logic in addition.
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Eq, PartialEq)]
pub struct Operation<T: Trait> {
    /// Type of this operation.
    pub operation: OperationType,
    /// The name being operated on.
    pub name: T::Name,
    /// The value it is being set to.
    pub value: T::Value,

    /// The sender of the name (who pays the name fee).
    sender: T::AccountId,
    /// The owner it is sent to.
    recipient: T::AccountId,

    /// The name fee to pay.
    fee: <T::Currency as Currency<T::AccountId>>::Balance,
}

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule {
        /// The main name -> data mapping.
        Names: map T::Name => Option<NameData<T>>;
        /// All names (as both the second key and the value) that may expire at
        /// the given block height (first key).  We use this so we can
        /// efficiently process expirations whenever we process a new block.
        /// When names are updated, they are not removed from here, though --
        /// so a name's expiration value in the core database overrules this
        /// index.
        Expirations: double_map T::BlockNumber, blake2_256(T::Name) => T::Name;
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
            Self::execute(data)?;
            Ok(())
        }

        /// Tries to transfer a name to a given recipient.  If the name does
        /// not exist, it will be registered directly to them with a default
        /// value.
        pub fn transfer(origin, name: T::Name, recipient: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let data = Self::check_assuming_signed(who, name, None, Some(recipient))?;
            Self::execute(data)?;
            Ok(())
        }

    }
}

impl<T: Trait> Module<T> {

    /// Returns a withdraw reasons value for the fee payment.
    fn withdraw_reasons() -> WithdrawReasons {
        let mut res = WithdrawReasons::none();
        res.set(WithdrawReason::Fee);
        res
    }

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

        let value = match value {
            None => old_value,
            Some(new_value) => new_value,
        };
        let recipient = match recipient {
            None => sender.clone(),
            Some(new_recipient) => new_recipient,
        };

        let mut op = Operation::<T> {
            operation: typ,
            name: name,
            value: value,
            sender: sender,
            recipient: recipient,
            fee: <T::Currency as Currency<T::AccountId>>::Balance::default(),
        };
        op.fee = match T::get_name_fee(&op) {
            None => return Err("operation violates name policy"),
            Some(f) => f,
        };

        /* Make sure that we can withdraw the name fee from the sender account.
           Note that ensure_can_withdraw does not by itself verify the
           amount against the free balance, but just that the new balance
           satisfies all locks in place.  Thus we have to do that ourselves.  */
        let new_balance = match T::Currency::free_balance(&op.sender).checked_sub(&op.fee) {
            None => return Err("insufficient balance for name fee"),
            Some(b) => b,
        };
        match T::Currency::ensure_can_withdraw(&op.sender, op.fee, Self::withdraw_reasons(), new_balance) {
            Err(_) => return Err("cannot withdraw name fee from sender"),
            Ok(_) => (),
        }

        Ok(op)
    }

    /// Executes the state change (and fires events) for a given name operation.
    /// This should be called after check_assuming_signed (passing its result),
    /// and when potential other checks have been done as well.
    ///
    /// This function may actually fail (return an error) if the fee withdrawal
    /// is not possible.  This can happen if some funds were spent externally
    /// between the call to check_assuming_signed and this function.  If that
    /// happens, then execute will be a noop.
    pub fn execute(op: Operation<T>) -> DispatchResult {
        /* As the very first step, handle the name fee.  This makes sure
           that if withdrawal fails, it will not cause any other changes.  */
        let imbalance = T::Currency::withdraw(&op.sender, op.fee,
                                              Self::withdraw_reasons(),
                                              ExistenceRequirement::AllowDeath)?;
        T::deposit_fee(imbalance);

        let expiration_blocks = T::get_expiration(&op);
        let expiration_height = match expiration_blocks {
            None => None,
            Some(b) => {
                /* In the strange case that we are told to use zero blocks for
                   expiration, make it at least one.  This ensures that we will
                   actually expire the name in the next block, and not end up
                   with an index entry from the past that will stick around
                   forever.  */
                let b = max(b, T::BlockNumber::from(1));
                Some(system::Module::<T>::block_number() + b)
            },
        };

        let data = NameData::<T> {
            value: op.value,
            owner: op.recipient,
            expiration: expiration_height,
        };

        <Names<T>>::insert(&op.name, &data);
        if let Some(h) = expiration_height {
            <Expirations<T>>::insert(h, &op.name, &op.name);
        }

        match op.operation {
            OperationType::Registration => {
                Self::deposit_event(RawEvent::NameRegistered(op.name.clone()));
            },
            OperationType::Update => (),
        }
        Self::deposit_event(RawEvent::NameUpdated(op.name, data));

        Ok(())
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

/// Module with unit tests.
#[cfg(test)]
mod tests;
