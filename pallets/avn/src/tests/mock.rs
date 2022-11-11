// Copyright 2022 Aventus Network Services (UK) Ltd.

#![cfg(test)]

use pallet_parachain_staking as staking;
use crate::{self as pallet_avn, *};
use frame_support::{
    parameter_types,
    traits::{GenesisBuild, ValidatorRegistration},
    weights::Weight,
    BasicExternalities,
    PalletId};
use frame_system as system;
use pallet_session as session;
use frame_system::RawOrigin;
use sp_core::{
    offchain::testing::{OffchainState, PendingRequest},
    H256,
    sr25519,
};
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, ConvertInto, IdentityLookup},
    Perbill,
};
use std::cell::RefCell;

pub type AccountId = <Test as system::Config>::AccountId;
pub type AuthorityId = <Test as Config>::AuthorityId;
pub type AVN = Pallet<Test>;
pub type Signature = sr25519::Signature;
pub type Balance = u128;

pub type PalletParachainStaking = staking::Pallet<Test>;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        ParachainStaking: staking::{Pallet, Call, Storage, Config<T>, Event<T>},
        Avn: pallet_avn::{Pallet, Storage},
    }
);

impl Config for Test {
    type AuthorityId = UintAuthorityId;
    type EthereumPublicKeyChecker = ();
    type NewSessionHandler = ();
    type DisabledValidatorChecker = ();
    type FinalisedBlockChecker = ();
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const ChallengePeriod: u64 = 2;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 0;
}

impl pallet_balances::Config for Test {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 4];
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinBlocksPerEra: u32 = 3;
    pub const RewardPaymentDelay: u32 = 2;
    pub const MinSelectedCandidates: u32 = 5;
    pub const MaxTopNominationsPerCandidate: u32 = 4;
    pub const MaxBottomNominationsPerCandidate: u32 = 4;
    pub const MaxNominationsPerNominator: u32 = 4;
    pub const MinNominationPerCollator: u128 = 3;
    pub const ErasPerGrowthPeriod: u32 = 2;
    pub const RewardPotId: PalletId = PalletId(*b"av/vamgr");
}

pub struct IsRegistered;
impl ValidatorRegistration<AccountId> for IsRegistered {
    fn is_registered(_id: &AccountId) -> bool {
        true
    }
}

impl staking::Config for Test {
    type Call = Call;
    type Event = Event;
    type Currency = Balances;
    type RewardPaymentDelay = RewardPaymentDelay;
    type MinBlocksPerEra = MinBlocksPerEra;
    type MinSelectedCandidates = MinSelectedCandidates;
    type MaxTopNominationsPerCandidate = MaxTopNominationsPerCandidate;
    type MaxBottomNominationsPerCandidate = MaxBottomNominationsPerCandidate;
    type MaxNominationsPerNominator = MaxNominationsPerNominator;
    type MinNominationPerCollator = MinNominationPerCollator;
    type RewardPotId = RewardPotId;
    type ErasPerGrowthPeriod = ErasPerGrowthPeriod;
    type ProcessedEventsChecker = ();
    type Public = AccountId;
    type Signature = Signature;
    type CollatorSessionRegistration = IsRegistered;
    type WeightInfo = ();
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}

thread_local! {
    // validator accounts (aka public addresses, public keys-ish)
    pub static VALIDATORS: RefCell<Option<Vec<u64>>> = RefCell::new(Some(vec![1, 2, 3]));
}

pub type SessionIndex = u32;

pub struct TestSessionManager;
impl session::SessionManager<u64> for TestSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<u64>> {
        VALIDATORS.with(|l| l.borrow_mut().take())
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

impl session::Config for Test {
    type SessionManager = TestSessionManager;
    type Keys = UintAuthorityId;
    type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
    type SessionHandler = (AVN,);
    type Event = ();
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type NextSessionRotation = session::PeriodicSessions<Period, Offset>;
    type WeightInfo = ();
}

pub struct ExtBuilder {
    pub storage: sp_runtime::Storage,
}

impl ExtBuilder {
    pub fn build_default() -> Self {
        let storage =
            frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
        Self { storage }
    }

    pub fn as_externality(self) -> sp_io::TestExternalities {
        let mut ext = sp_io::TestExternalities::from(self.storage);
        // Events do not get emitted on block 0, so we increment the block here
        ext.execute_with(|| frame_system::Pallet::<Test>::set_block_number(1u32.into()));
        ext
    }

    pub fn with_validators(mut self) -> Self {
        let validators: Vec<u64> = VALIDATORS.with(|l| l.borrow_mut().take().unwrap());

        BasicExternalities::execute_with_storage(&mut self.storage, || {
            for ref k in &validators {
                frame_system::Pallet::<Test>::inc_providers(k);
            }
        });

        let _ = pallet_session::GenesisConfig::<Test> {
            keys: validators.into_iter().map(|v| (v, v, UintAuthorityId(v))).collect(),
        }
        .assimilate_storage(&mut self.storage);
        self
    }
}

/************* Test helpers ************ */

#[allow(dead_code)]
pub fn keys_setup_return_good_validator(
) -> Validator<<Test as Config>::AuthorityId, AccountId> {
    let validators = AVN::validators(); // Validators are tuples (UintAuthorityId(int), int)
    assert_eq!(validators[0], Validator { account_id: 1, key: UintAuthorityId(1) });
    assert_eq!(validators[2], Validator { account_id: 3, key: UintAuthorityId(3) });
    assert_eq!(validators.len(), 3);

    // AuthorityId type for Test is UintAuthorityId
    let keys: Vec<UintAuthorityId> = validators.into_iter().map(|v| v.key).collect();
    UintAuthorityId::set_all_keys(keys); // Keys in the setup are either () or (1,2,3). See VALIDATORS.
    let current_node_validator = AVN::get_validator_for_current_node().unwrap(); // filters validators() to just those corresponding to this validator
    assert_eq!(current_node_validator.key, UintAuthorityId(1));
    assert_eq!(current_node_validator.account_id, 1);

    assert_eq!(current_node_validator, Validator { account_id: 1, key: UintAuthorityId(1) });

    return current_node_validator
}

#[allow(dead_code)]
pub fn bad_authority() -> Validator<<Test as Config>::AuthorityId, AccountId> {
    let validator = Validator { account_id: 0, key: UintAuthorityId(0) };

    return validator
}

#[allow(dead_code)]
pub fn mock_get_request(state: &mut OffchainState, url_param: String, response: Option<Vec<u8>>) {
    let mut url = "http://127.0.0.1:2020/eth/sign/".to_string();
    url.push_str(&url_param);

    state.expect_request(PendingRequest {
        method: "GET".into(),
        uri: url.into(),
        response,
        headers: vec![],
        sent: true,
        ..Default::default()
    });
}

#[allow(dead_code)]
pub fn mock_post_request(state: &mut OffchainState, body: Vec<u8>, response: Option<Vec<u8>>) {
    state.expect_request(PendingRequest {
        method: "POST".into(),
        uri: "http://127.0.0.1:2020/eth/send".into(),
        response,
        headers: vec![],
        body,
        sent: true,
        ..Default::default()
    });
}

pub fn setup_keys()  {
    let validators = AVN::validators();
    let keys: Vec<UintAuthorityId> = validators.into_iter().map(|v| v.key).collect();

    UintAuthorityId::set_all_keys(keys);
}

fn set_session_keys(collator_id: &AccountId, auth_id: AuthorityId) {
    pallet_session::NextKeys::<Test>::insert::<AccountId, UintAuthorityId>(
        *collator_id,
        auth_id
    );
}

// pub fn register_collator_candidate(collator_id: &AccountId, auth_id: AuthorityId) {
//     set_session_keys(collator_id, auth_id);
//     let _ = PalletParachainStaking::join_candidates(
//         RawOrigin::Signed(
//             collator_id.clone(),
//         ).into(),
//         10u128,
//         0u32
//     );
// }

// pub fn remove_collator_candidate(collator_id: &AccountId) {
//     let _ = PalletCollatorSelection::leave_intent(
//         RawOrigin::Signed(
//             collator_id.clone(),
//         ).into(),
//     );
// }