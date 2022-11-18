// Copyright 2022 Aventus Network Services (UK) Ltd.

use crate::{
    // assert_last_event,
    mock::*,
    // Bond, CollatorStatus, Error, Event, NominationScheduledRequests, NominatorAdded,
    // NominatorState, NominatorStatus, NOMINATOR_LOCK_ID,
};
use sp_runtime::testing::UintAuthorityId;
use frame_support::assert_ok;

fn change_validators_good() {
    VALIDATOR_SEEDS.with(|v| {
        let mut v = v.borrow_mut();
        *v = Some(vec![1, 2]);
        Some(v.clone())
    });

    advance_session_and_force_new_validators();
}

fn change_validators_empty() {
    VALIDATOR_SEEDS.with(|v| {
        let mut v = v.borrow_mut();
        *v = Some(vec![]);
        Some(v.clone())
    });

    advance_session_and_force_new_validators();
}

fn advance_session_no_validators_change() {
    VALIDATOR_SEEDS.with(|v| {
        let mut v = v.borrow_mut();
        *v = None;
        Some(v.clone())
    });

    advance_session_and_force_new_validators();
}

fn advance_session_and_force_new_validators() {
    // need to do it twice for the change to take effect
    advance_session();
    advance_session();
}

// TODO [TYPE: test refactoring][PRI: LOW]:  update this function to work with the mock builder
// pattern Currently, a straightforward replacement of the test setup leads to an error on the
// assert_eq!

// fn advance_session() {
//     // let now = System::block_number().max(1);
//     // System::set_block_number(now + 1);
//     // Session::rotate_session();
//     // assert_eq!(Session::current_index(), (now / Period::get()) as u32);
//     let now = System::block_number().max(1);
//     <crate::parachain_staking::ForceNewEra<TestRuntime>>::put(true);

//     Balances::on_finalize(System::block_number());
//     System::on_finalize(System::block_number());
//     System::set_block_number(now + 1);
//     System::on_initialize(System::block_number());
//     Balances::on_initialize(System::block_number());
//     Session::on_initialize(System::block_number());
//     ParachainStaking::on_initialize(System::block_number());
// }

fn avn_known_collators() -> sp_application_crypto::Vec<sp_avn_common::event_types::Validator<AuthorityId, sp_core::sr25519::Public>> {
    return AVN::validators();
}

fn add_collator_candidate(id: AccountId, auth_id: u64) {
    let new_candidate_id = id;
    let auth_id = UintAuthorityId(auth_id);

    add_collator(&new_candidate_id, auth_id);
}

mod chain_started_with_invulnerable_collators_only {
    use super::*;
    fn setup_invulnerable_collators() -> sp_io::TestExternalities {
        // let initial_validators = vec![(TestAccount::derive_account_id(1), 10000), (TestAccount::derive_account_id(2), 10000), (TestAccount::derive_account_id(3), 10000)];
        let mut ext = ExtBuilder::build_default().with_balances().with_validators().with_parachain_staking().as_externality();
        ext
    }

    // #[test]
    // fn all_and_only_invulnerable_collators_are_registered_with_avn_pallet_at_startup()
    // {
    //     let mut ext = setup_invulnerable_collators();

    //     ext.execute_with(|| {
    //         assert!(
    //             AVN::validators() ==
    //             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
    //         );
    //     });
    // }

    // #[test]
    // fn if_no_changes_between_sessions_then_avn_knows_same_collators()
    // {
    //     let mut ext = setup_invulnerable_collators();

    //     ext.execute_with(|| {
    //         let initial_collators = avn_known_collators();
    //         advance_session();
    //         let current_collators = avn_known_collators();
    //         advance_session();
    //         let final_collators = avn_known_collators();

    //         assert_eq!(initial_collators, current_collators);
    //         assert_eq!(current_collators, final_collators);
    //     });
    // }

    mod when_new_candidate_registers {
        use super::*;

        // #[test]
        // fn then_no_change_visible_in_following_session(){
        //     let mut ext = setup_invulnerable_collators();
        //     let added_valditator = TestAccount::derive_validator(4);

        //     ext.execute_with(|| {
        //         let initial_collators = avn_known_collators();
        //         add_collator_candidate(added_valditator.account_id, 4);

        //         advance_session();

        //         let final_collators = avn_known_collators();
        //         assert_eq!(initial_collators, final_collators);
        //     })
        // }

        #[test]
        fn then_avn_knows_collator_after_two_sessions(){
            let mut ext = setup_invulnerable_collators();
            let added_valditator = TestAccount::derive_validator(4);

            ext.execute_with(|| {
                // add_collator(added_valditator.account_id);
                add_collator_candidate(added_valditator.account_id, 4);
                advance_session();
                advance_session();

                let test_session_validators = 	Session::validators();
                println!("\nFINAL SESSION VALIDATORS: {:?}\n", &test_session_validators);

                let final_collators = avn_known_collators();

                assert_eq!(final_collators,
                    vec![
                    TestAccount::derive_validator(3),
                    TestAccount::derive_validator(4),
                    TestAccount::derive_validator(1),
                    TestAccount::derive_validator(2),
                    ]);
            })
        }
        // #[test]
        // fn then_avn_knows_collator_after_two_sessions(){
        //     let mut ext = setup_invulnerable_collators();
        //     ext.execute_with(|| {
        //         let initial_collators = avn_known_collators();
        //         add_collator_candidate(4, 4);

        //         advance_session();
        //         advance_session();

        //         let final_collators = avn_known_collators();

        //         assert_eq!(final_collators, vec![
        //             Validator { account_id: 1, key: UintAuthorityId(1) },
        //             Validator { account_id: 2, key: UintAuthorityId(2) },
        //             Validator { account_id: 3, key: UintAuthorityId(3) },
        //             Validator { account_id: 4, key: UintAuthorityId(4) }
        //         ]);
        //         // let ic = initial_collators.clone();
        //         // let ia = ic.push(Validator { account_id: 4, key: UintAuthorityId(4) });
        //         // assert_eq!(initial_collators.push(Validator { account_id: 4, key: UintAuthorityId(4) }), final_collators);
        //         // assert_eq!(ia, final_collators);
        //     })
        // }

        // #[test]
        // fn with_new_key_then_avn_information_is_updated(){
        //     let mut ext = setup_invulnerable_collators();
        //     ext.execute_with(|| {
        //         add_collator_candidate(3, 4);

        //         advance_session();
        //         advance_session();

        //         let final_collators = avn_known_collators();
        //         assert_eq!(final_collators, vec![
        //             Validator { account_id: 1, key: UintAuthorityId(1) },
        //             Validator { account_id: 2, key: UintAuthorityId(2) },
        //             Validator { account_id: 3, key: UintAuthorityId(4) }
        //         ]);
        //     })
        // }
    }
}

// #[test]
// // *changed is true but with the same validators: keys list has not changed
// fn keys_populated_correctly_new_session_same_validators_change() {
//     let mut ext = ExtBuilder::build_default().with_validators().as_externality();
//     ext.execute_with(|| {
//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
//         );

//         advance_session();

//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
//         );
//     });
// }


// #[test]
// // * changed is true: Ensure that the keys have been updated
// fn keys_populated_correctly_new_session_with_good_change() {
//     let mut ext = ExtBuilder::build_default().with_validators().as_externality();
//     ext.execute_with(|| {
//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
//         );

//         change_validators_good();

//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2)]
//         );
//     });
// }

// #[test]
// // * changed is true: Ensure that the keys have been updated
// fn keys_populated_correctly_new_session_with_empty_change() {
//     let mut ext = ExtBuilder::build_default().with_validators().as_externality();
//     ext.execute_with(|| {
//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
//         );

//         change_validators_empty();

//         assert!(AVN::validators() == vec![]);
//     });
// }

// #[test]
// // * changed is false: keys list has not changed
// fn keys_populated_correctly_new_session_with_no_change() {
//     let mut ext = ExtBuilder::build_default().with_validators().as_externality();
//     ext.execute_with(|| {
//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
//         );

//         advance_session_no_validators_change();

//         assert!(
//             AVN::validators() ==
//             vec![ TestAccount::derive_validator(1), TestAccount::derive_validator(2), TestAccount::derive_validator(3)]
//         );
//     });
// }
