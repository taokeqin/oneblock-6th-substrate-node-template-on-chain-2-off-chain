#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode};
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
	SigningTypes,
};
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	offchain::{http, storage::StorageValueRef, Duration},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug,
};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}
//const INDEX_KEY: str = b"url::index";
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, sp_io::offchain_index};
	use frame_system::pallet_prelude::*;
	use serde::Deserialize;
	use sp_std::vec::Vec;
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct Payload<Public> {
		number: u64,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for Payload<T::Public> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}
	const ONCHAIN_TX_KEY: &[u8] = b"my_pallet::indexing1";

	#[derive(Debug, Deserialize, Encode, Decode, Default)]
	struct IndexingData(u64);

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	// pub trait Config: frame_system::Config +
	// frame_system::offchain::SendTransactionTypes<Call<Self>> {
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { something: u32, who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { something, who });
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}

		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn unsigned_extrinsic_with_signed_payload(
			origin: OriginFor<T>,
			payload: Payload<T::Public>,
			_signature: T::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			log::info!(
				"OCW ==> in call unsigned_extrinsic_with_signed_payload: {:?}",
				payload.number
			);
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn set_xkcd_id(origin: OriginFor<T>, xkcd_id: u64) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			let key = Self::derived_key(frame_system::Pallet::<T>::block_number());
			//let block_number = frame_system::Pallet::<T>::block_number();
			log::info!("+++extrinsic key: {:?}", key);
			let data = IndexingData(xkcd_id);
			offchain_index::set(&key, &data.encode());
			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			const UNSIGNED_TXS_PRIORITY: u64 = 100;
			let valid_tx = |provide| {
				ValidTransaction::with_tag_prefix("my-pallet")
					.priority(UNSIGNED_TXS_PRIORITY) // please define `UNSIGNED_TXS_PRIORITY` before this line
					.and_provides([&provide])
					.longevity(3)
					.propagate(true)
					.build()
			};

			// match call {
			// 	Call::submit_data_unsigned { key: _ } => valid_tx(b"my_unsigned_tx".to_vec()),
			// 	_ => InvalidTransaction::Call.into(),
			// }

			match call {
				Call::unsigned_extrinsic_with_signed_payload { ref payload, ref signature } => {
					if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
						return InvalidTransaction::BadProof.into()
					}
					valid_tx(b"unsigned_extrinsic_with_signed_payload".to_vec())
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn derived_key(block_number: T::BlockNumber) -> Vec<u8> {
			block_number.using_encoded(|encoded_bn| {
				ONCHAIN_TX_KEY
					.clone()
					.into_iter()
					.chain(b"/".into_iter())
					.chain(encoded_bn)
					.copied()
					.collect::<Vec<u8>>()
			})
		}
	}

	// impl fetch_url using offchain http

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain worker entry point.
		fn offchain_worker(block_number: T::BlockNumber) {
			// let value: u64 = 42;
			// // This is your call to on-chain extrinsic together with any necessary parameters.
			// let call = Call::submit_data_unsigned { key: value };

			// // `submit_unsigned_transaction` returns a type of `Result<(), ()>`
			// //	 ref: https://paritytech.github.io/substrate/master/frame_system/offchain/struct.SubmitTransaction.html
			// _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			// 	.map_err(|_| {
			// 	log::error!("OCW ==> Failed in offchain_unsigned_tx");
			// });
			log::info!(
				"===OCW=== Hello World from offchain workers! block number: {:?}",
				block_number
			);

			let key = Self::derived_key(block_number);
			log::info!("===OCW=== offchain_worker key: {:?}", key);
			let storage_ref = StorageValueRef::persistent(&key);
			if let Ok(Some(data)) = storage_ref.get::<IndexingData>() {
				log::info!("===OCW=== local storage data: {:?}", data);
			} else {
				log::info!("===OCW=== Error reading from local storage.");
			}

			// use http to request image
			let image_url = "https://imgs.xkcd.com/comics/thats_what_she_said.png";
			log::info!("===OCW=== image_url: {:?}", image_url);
			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
			let request = http::Request::get(image_url);
			let pending =
				request.deadline(deadline).send().map_err(|_| http::Error::IoError).unwrap();
			let response = pending
				.try_wait(deadline)
				.map_err(|_| http::Error::DeadlineReached)
				.unwrap()
				.unwrap();
			// Let's check the status code before we proceed to reading the response.
			if response.code != 200 {
				log::warn!("Unexpected status code: {}", response.code);
			}
			log::info!("===OCW=== response code: {:?}", response.code);

			let number: u64 = response.body().collect::<Vec<u8>>().len() as u64;
			// Retrieve the signer to sign the payload
			let signer = Signer::<T, T::AuthorityId>::any_account();

			// `send_unsigned_transaction` is returning a type of `Option<(Account<T>, Result<(),
			// ()>)>`. 	 The returned result means:
			// 	 - `None`: no account is available for sending transaction
			// 	 - `Some((account, Ok(())))`: transaction is successfully sent
			// 	 - `Some((account, Err(())))`: error occurred when sending the transaction
			if let Some((_, res)) = signer.send_unsigned_transaction(
				// this line is to prepare and return payload
				|acct| Payload { number, public: acct.public.clone() },
				|payload, signature| Call::unsigned_extrinsic_with_signed_payload {
					payload,
					signature,
				},
			) {
				match res {
					Ok(()) => {
						log::info!("OCW ==> unsigned tx with signed payload successfully sent.");
					},
					Err(()) => {
						log::error!("OCW ==> sending unsigned tx with signed payload failed.");
					},
				};
			} else {
				// The case of `None`: no account is available for sending
				log::error!("OCW ==> No local account available");
			}
		}
	}
}
