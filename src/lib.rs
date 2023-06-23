#![cfg_attr(not(feature = "std"), no_std)]

//! # Multitoken Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! Implements the ERC1155 token standard for partially fungible tokens.
//! See https://docs.openzeppelin.com/contracts/3.x/erc1155

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
//pub mod weights;
//pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use alloc::vec;
    use alloc::vec::Vec;
    use core::fmt::Debug;

    use codec::Codec;
    use core::default::Default;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::AtLeast32BitUnsigned;
    use sp_runtime::FixedPointOperand;

    use super::*;

    pub trait Next {
        fn next(&self) -> Self;
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Identifier for the collection of item.
        type CollectionId: Member + Parameter + MaxEncodedLen + Copy + Default + Next;

        /// Identifier for numerical amounts.
        type Amount: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Codec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + MaxEncodedLen
            + TypeInfo
            + FixedPointOperand;

        //// The weight information for this pallet.
        // type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new collection has been created
        CollectionCreated {
            id: T::CollectionId,
            owner: T::AccountId,
        },
        /// Emitted when `value` tokens of token type `id` are transferred from `from` to `to` by `operator`.
        TransferSingle {
            operator: T::AccountId,
            from: Option<T::AccountId>,
            to: Option<T::AccountId>,
            id: T::CollectionId,
            value: T::Amount,
        },
        /// Equivalent to multiple `TransferSingle` events, where `operator`, `from` and `to` are the same for all transfers.
        TransferBatch {
            operator: T::AccountId,
            from: Option<T::AccountId>,
            to: Option<T::AccountId>,
            ids: Vec<T::CollectionId>,
            values: Vec<T::Amount>,
        },
        /// Emitted when `account` grants or revokes permission to `operator` to transfer their tokens, according to `approved`.
        ApprovalForAll {
            account: T::AccountId,
            operator: T::AccountId,
            approved: bool,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Operator not allowed to perform the action.
        InvalidOperator,
        /// Operator does not have sufficient rights to move funds.
        InsufficientApprovalForAll,
        /// Arrays have different length when invoking extrinsics.
        InvalidArrayLength,
        /// User has not enough balance of a given token collection.
        InsufficientBalance,
        /// The collection does not exist.
        CollectionDoesNotExist,
        /// The account is not the one that created the collection.
        InvalidOwner,
    }

    /// Stores the `CollectionId` that is going to be used for the next collection.
    /// This gets incremented whenever a new collection is created.
    #[pallet::storage]
    #[pallet::getter(fn next_collection_id)]
    pub type NextCollectionId<T: Config> = StorageValue<_, T::CollectionId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn collections)]
    pub type Collections<T: Config> =
        CountedStorageMap<_, Twox64Concat, T::CollectionId, T::AccountId, OptionQuery>;

    /// Maps collection to account balance.
    #[pallet::storage]
    #[pallet::getter(fn balances)]
    pub type Balances<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::CollectionId,
        Twox64Concat,
        T::AccountId,
        T::Amount,
        OptionQuery,
    >;

    /// Maps owner to operator approval.
    #[pallet::storage]
    #[pallet::getter(fn operator_approvals)]
    pub type OperatorApprovals<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Grants or revokes permission to `operator` to transfer the caller's tokens, according to `approved`.
        #[pallet::call_index(0)]
        #[pallet::weight({0})]
        pub fn set_approval_for_all(
            origin: OriginFor<T>,
            operator: T::AccountId,
            approved: bool,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            ensure!(owner != operator, Error::<T>::InvalidOperator,);
            OperatorApprovals::<T>::insert(owner.clone(), operator.clone(), approved);
            Self::deposit_event(Event::<T>::ApprovalForAll {
                account: owner,
                operator,
                approved,
            });
            Ok(())
        }

        /// Transfers `amount` tokens of token type `id` from `from` to `to`.
        #[pallet::call_index(1)]
        #[pallet::weight({0})]
        pub fn safe_transfer_from(
            origin: OriginFor<T>,
            from: T::AccountId,
            to: T::AccountId,
            id: T::CollectionId,
            amount: T::Amount,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(from != sender, Error::<T>::InsufficientApprovalForAll);
            Self::update(sender, Some(from), Some(to), vec![id], vec![amount])
        }

        /// Version of `safe_transfer_from`.
        #[pallet::call_index(2)]
        #[pallet::weight({0})]
        pub fn safe_batch_transfer_from(
            origin: OriginFor<T>,
            from: T::AccountId,
            to: T::AccountId,
            ids: Vec<T::CollectionId>,
            amounts: Vec<T::Amount>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(
                from == sender || Self::is_approved_for_all(&from, &sender),
                Error::<T>::InsufficientApprovalForAll
            );
            Self::update(sender, Some(from), Some(to), ids, amounts)
        }

        /// Mints `amount` new tokens of collection `id` to user `to`.
        /// Only the root account can perform this action.
        #[pallet::call_index(3)]
        #[pallet::weight({0})]
        pub fn mint(
            origin: OriginFor<T>,
            to: T::AccountId,
            id: T::CollectionId,
            amount: T::Amount,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let owner = Collections::<T>::get(id);
            ensure!(owner.is_some(), Error::<T>::CollectionDoesNotExist);
            ensure!(owner.unwrap() == sender, Error::<T>::InvalidOwner);
            Self::update(sender, None, Some(to), vec![id], vec![amount])
        }

        /// Version of `mint`.
        #[pallet::call_index(4)]
        #[pallet::weight({0})]
        pub fn mint_batch(
            origin: OriginFor<T>,
            to: T::AccountId,
            ids: Vec<T::CollectionId>,
            amounts: Vec<T::Amount>,
        ) -> DispatchResult {
            ensure_root(origin.clone())?;
            let sender = ensure_signed(origin)?;
            Self::update(sender, None, Some(to), ids, amounts)
        }

        /// Burns `amount` of collection `id` that belong to `origin`.
        #[pallet::call_index(5)]
        #[pallet::weight({0})]
        pub fn burn(
            origin: OriginFor<T>,
            id: T::CollectionId,
            amount: T::Amount,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::update(sender.clone(), Some(sender), None, vec![id], vec![amount])
        }

        /// Version of `burn`
        #[pallet::call_index(6)]
        #[pallet::weight({0})]
        pub fn burn_batch(
            origin: OriginFor<T>,
            ids: Vec<T::CollectionId>,
            amounts: Vec<T::Amount>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::update(sender.clone(), Some(sender), None, ids, amounts)
        }

        /// Creates a new collection
        #[pallet::call_index(7)]
        #[pallet::weight({0})]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let collection_id = NextCollectionId::<T>::get();
            Collections::<T>::insert(collection_id, sender.clone());
            NextCollectionId::<T>::set(collection_id.next());
            Self::deposit_event(Event::<T>::CollectionCreated {
                id: collection_id,
                owner: sender,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Transfers `amount` tokens of token type `id` from `from` to `to`. Will mint (or burn) if `from` (or `to`) is `None`.
        fn update(
            operator: T::AccountId,
            from: Option<T::AccountId>,
            to: Option<T::AccountId>,
            ids: Vec<T::CollectionId>,
            amounts: Vec<T::Amount>,
        ) -> DispatchResult {
            ensure!(ids.len() == amounts.len(), Error::<T>::InvalidArrayLength);
            for i in 0..ids.len() {
                let id = ids[i];
                let amount = amounts[i];

                if let Some(from) = &from {
                    let from_balance =
                        Balances::<T>::get(id, from).ok_or(<Error<T>>::CollectionDoesNotExist)?;
                    ensure!(from_balance >= amount, Error::<T>::InsufficientBalance);
                    Balances::<T>::insert(id, from, from_balance - amount);
                }

                if let Some(to) = &to {
                    Balances::<T>::insert(id, to, amount);
                }
            }

            if ids.len() == 1 {
                Self::deposit_event(Event::<T>::TransferSingle {
                    operator,
                    from,
                    to,
                    id: ids[0],
                    value: amounts[0],
                });
            } else {
                Self::deposit_event(Event::<T>::TransferBatch {
                    operator,
                    from,
                    to,
                    ids,
                    values: amounts,
                });
            }
            Ok(())
        }

        /// Returns the amount of tokens of token type `id` owned by `account`.
        pub fn balance_of(account: &T::AccountId, id: &T::CollectionId) -> T::Amount {
            Balances::<T>::get(id, account).unwrap_or_default()
        }

        /// Version of `balance_of`.
        pub fn balance_of_batch(
            accounts: &Vec<T::AccountId>,
            ids: &Vec<T::CollectionId>,
        ) -> Option<Vec<T::Amount>> {
            let len = accounts.len();
            if len != ids.len() {
                return None;
            }
            let mut balances = Vec::with_capacity(len);
            for i in 0..len {
                balances[i] = Self::balance_of(&accounts[i], &ids[i]);
            }
            Some(balances)
        }

        /// Returns true if `operator` is approved to transfer `account`'s tokens.
        pub fn is_approved_for_all(account: &T::AccountId, operator: &T::AccountId) -> bool {
            OperatorApprovals::<T>::get(account, operator)
        }

        pub fn all_collections() -> Vec<(T::CollectionId, T::AccountId)> {
            Collections::<T>::iter().collect()
        }
    }
}
