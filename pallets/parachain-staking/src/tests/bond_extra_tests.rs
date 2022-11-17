//Copyright 2022 Aventus Network Services.bond_extra

#![cfg(test)]

use crate::{
    assert_event_emitted, assert_last_event, encode_signed_bond_extra_params,
    mock::{
        build_proof, sign, AccountId, AvnProxy, Call as MockCall, Event as MetaEvent, ExtBuilder,
        MinNominationPerCollator, Origin, ParachainStaking, Signature, Staker, Test, TestAccount,
    },
    Config, Error, Event, NominatorAdded, Proof, StaticLookup,
};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use frame_system::{self as system, RawOrigin};
use sp_runtime::traits::Zero;

fn to_acc_id(id: u64) -> AccountId {
    return TestAccount::new(id).account_id()
}

mod proxy_signed_bond_extra {
    use super::*;

    fn create_call_for_bond_extra(
        staker: &Staker,
        sender_nonce: u64,
        extra_amount: u128,
    ) -> Box<<Test as Config>::Call> {
        let proof = create_proof_for_signed_bond_extra(sender_nonce, staker, &extra_amount);

        return Box::new(MockCall::ParachainStaking(super::super::Call::<Test>::signed_bond_extra {
            proof,
            extra_amount,
        }))
    }

    fn create_proof_for_signed_bond_extra(
        sender_nonce: u64,
        staker: &Staker,
        extra_amount: &u128,
    ) -> Proof<Signature, AccountId> {
        let data_to_sign = encode_signed_bond_extra_params::<Test>(
            staker.relayer.clone(),
            extra_amount,
            sender_nonce,
        );

        let signature = sign(&staker.key_pair, &data_to_sign);
        return build_proof(&staker.account_id, &staker.relayer, signature)
    }

    #[test]
    fn succeeds_with_good_parameters() {
        let collator_1 = to_acc_id(1u64);
        let collator_2 = to_acc_id(2u64);
        let staker: Staker = Default::default();
        let initial_stake = 10;
        let initial_balance = 10000;
        ExtBuilder::default()
            .with_balances(vec![
                (collator_1, initial_balance),
                (collator_2, initial_balance),
                (staker.account_id, initial_balance),
                (staker.relayer, initial_balance),
            ])
            .with_candidates(vec![(collator_1, initial_stake), (collator_2, initial_stake)])
            .with_nominations(vec![
                (staker.account_id, collator_1, initial_stake),
                (staker.account_id, collator_2, initial_stake),
            ])
            .build()
            .execute_with(|| {
                let collators = ParachainStaking::selected_candidates();
                let min_user_stake = MinNominationPerCollator::get();
                let initial_total_stake_on_chain = ParachainStaking::total();

                // Pick an amount that is not perfectly divisible by the number of collators
                let dust = 1u128;
                let amount_to_topup = (min_user_stake * 2u128) + dust;
                let nonce = ParachainStaking::proxy_nonce(staker.account_id);
                let bond_extra_call = create_call_for_bond_extra(&staker, nonce, amount_to_topup);
                assert_ok!(AvnProxy::proxy(Origin::signed(staker.relayer), bond_extra_call, None));

                // The staker state has also been updated
                let staker_state = ParachainStaking::nominator_state(staker.account_id).unwrap();
                assert_eq!(staker_state.total(), initial_stake * 2 + amount_to_topup);

                // Each collator has been topped up by the expected amount
                for (index, collator) in collators.into_iter().enumerate() {
                    // We should have one event per collator. One of the collators gets any
                    // remaining dust.
                    let mut topup = min_user_stake;
                    if index == 1 {
                        topup = min_user_stake + dust;
                    }

                    assert_event_emitted!(Event::NominationIncreased {
                        nominator: staker.account_id,
                        candidate: collator,
                        amount: topup,
                        in_top: true
                    });

                    // Staker state reflects the new nomination for each collator
                    assert_eq!(staker_state.nominations.0[index].owner, collator);
                    assert_eq!(staker_state.nominations.0[index].amount, initial_stake + topup);

                    // Collator state has been updated
                    let collator_state = ParachainStaking::candidate_info(collator).unwrap();
                    assert_eq!(collator_state.total_counted, initial_stake + initial_stake + topup);

                    // Collator nominations have also been updated
                    let top_nominations = ParachainStaking::top_nominations(collator).unwrap();
                    assert_eq!(top_nominations.total, initial_stake + topup);
                }

                // The staker free balance has been reduced
                assert_eq!(
                    ParachainStaking::get_nominator_stakable_free_balance(&staker.account_id),
                    10000 - (initial_stake * 2 + amount_to_topup)
                );

                // The total amount staked on chain should increase
                assert_eq!(
                    initial_total_stake_on_chain + amount_to_topup,
                    ParachainStaking::total()
                );

                // Nonce has increased
                assert_eq!(ParachainStaking::proxy_nonce(staker.account_id), nonce + 1);
            })
    }

    mod fails_when {
        use super::*;

        #[test]
        fn extrinsic_is_unsigned() {
            let collator_1 = to_acc_id(1u64);
            let collator_2 = to_acc_id(2u64);
            let staker: Staker = Default::default();
            ExtBuilder::default()
                .with_balances(vec![
                    (collator_1, 10000),
                    (collator_2, 10000),
                    (staker.account_id, 10000),
                    (staker.relayer, 10000),
                ])
                .with_candidates(vec![(collator_1, 10), (collator_2, 10)])
                .with_nominations(vec![
                    (staker.account_id, collator_1, 10),
                    (staker.account_id, collator_2, 10),
                ])
                .build()
                .execute_with(|| {
                    let amount_to_topup = MinNominationPerCollator::get() * 2u128;
                    let nonce = ParachainStaking::proxy_nonce(staker.account_id);
                    let bond_extra_call =
                        create_call_for_bond_extra(&staker, nonce, amount_to_topup);

                    assert_noop!(
                        AvnProxy::proxy(RawOrigin::None.into(), bond_extra_call, None),
                        BadOrigin
                    );
                });
        }

        #[test]
        fn staker_does_not_have_enough_funds() {
            let collator_1 = to_acc_id(1u64);
            let collator_2 = to_acc_id(2u64);
            let staker: Staker = Default::default();
            let staker_balance = 100;
            let initial_stake = 10;
            ExtBuilder::default()
                .with_balances(vec![
                    (collator_1, 10000),
                    (collator_2, 10000),
                    (staker.account_id, staker_balance),
                    (staker.relayer, 10000),
                ])
                .with_candidates(vec![(collator_1, initial_stake), (collator_2, initial_stake)])
                .with_nominations(vec![
                    (staker.account_id, collator_1, initial_stake),
                    (staker.account_id, collator_2, initial_stake),
                ])
                .build()
                .execute_with(|| {
                    let bad_amount_to_stake = staker_balance + 1;
                    let nonce = ParachainStaking::proxy_nonce(staker.account_id);

                    // Make sure 'bad_amount' is over the minimum allowed.
                    assert!(bad_amount_to_stake > MinNominationPerCollator::get() * 2u128);

                    // Make sure staker has less than they are attempting to stake
                    assert!(staker_balance < bad_amount_to_stake);

                    let bond_extra_call =
                        create_call_for_bond_extra(&staker, nonce, bad_amount_to_stake);

                    assert_noop!(
                        AvnProxy::proxy(Origin::signed(staker.relayer), bond_extra_call, None),
                        Error::<Test>::InsufficientBalance
                    );
                });
        }

        #[test]
        fn stake_is_less_than_min_allowed() {
            let collator_1 = to_acc_id(1u64);
            let collator_2 = to_acc_id(2u64);
            let staker: Staker = Default::default();
            let staker_balance = 10000;
            ExtBuilder::default()
                .with_balances(vec![
                    (collator_1, 10000),
                    (collator_2, 10000),
                    (staker.account_id, staker_balance),
                    (staker.relayer, 10000),
                ])
                .with_candidates(vec![(collator_1, 10), (collator_2, 10)])
                .with_nominations(vec![
                    (staker.account_id, collator_1, 10),
                    (staker.account_id, collator_2, 10),
                ])
                .build()
                .execute_with(|| {
                    let min_allowed_amount_to_stake = MinNominationPerCollator::get() * 2u128;
                    let bad_stake_amount = min_allowed_amount_to_stake - 1;

                    // Show that the staker has enough funds to cover the stake
                    assert!(staker_balance > bad_stake_amount);

                    let nonce = ParachainStaking::proxy_nonce(staker.account_id);
                    let bond_extra_call =
                        create_call_for_bond_extra(&staker, nonce, bad_stake_amount);

                    assert_noop!(
                        AvnProxy::proxy(Origin::signed(staker.relayer), bond_extra_call, None),
                        Error::<Test>::NominationBelowMin
                    );
                });
        }
    }
}

// NOMINATOR BOND EXTRA

#[test]
fn nominator_bond_extra_reserves_balance() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 15)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 10)])
        .build()
        .execute_with(|| {
            assert_eq!(ParachainStaking::get_nominator_stakable_free_balance(&account_id_2), 5);
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
            assert_eq!(ParachainStaking::get_nominator_stakable_free_balance(&account_id_2), 0);
        });
}

#[test]
fn nominator_bond_extra_increases_total_staked() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 15)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 10)])
        .build()
        .execute_with(|| {
            assert_eq!(ParachainStaking::total(), 40);
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
            assert_eq!(ParachainStaking::total(), 45);
        });
}

#[test]
fn nominator_bond_extra_updates_nominator_state() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 15)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 10)])
        .build()
        .execute_with(|| {
            assert_eq!(
                ParachainStaking::nominator_state(account_id_2).expect("exists").total(),
                10
            );
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
            assert_eq!(
                ParachainStaking::nominator_state(account_id_2).expect("exists").total(),
                15
            );
        });
}

#[test]
fn nominator_bond_extra_updates_candidate_state_top_nominations() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 15)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 10)])
        .build()
        .execute_with(|| {
            assert_eq!(
                ParachainStaking::top_nominations(account_id).unwrap().nominations[0].owner,
                account_id_2
            );
            assert_eq!(
                ParachainStaking::top_nominations(account_id).unwrap().nominations[0].amount,
                10
            );
            assert_eq!(ParachainStaking::top_nominations(account_id).unwrap().total, 10);
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
            assert_eq!(
                ParachainStaking::top_nominations(account_id).unwrap().nominations[0].owner,
                account_id_2
            );
            assert_eq!(
                ParachainStaking::top_nominations(account_id).unwrap().nominations[0].amount,
                15
            );
            assert_eq!(ParachainStaking::top_nominations(account_id).unwrap().total, 15);
        });
}

#[test]
fn nominator_bond_extra_updates_candidate_state_bottom_nominations() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![
            (account_id, 30),
            (account_id_2, 20),
            (to_acc_id(3), 20),
            (to_acc_id(4), 20),
            (to_acc_id(5), 20),
            (to_acc_id(6), 20),
        ])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![
            (account_id_2, account_id, 10),
            (to_acc_id(3), account_id, 20),
            (to_acc_id(4), account_id, 20),
            (to_acc_id(5), account_id, 20),
            (to_acc_id(6), account_id, 20),
        ])
        .build()
        .execute_with(|| {
            assert_eq!(
                ParachainStaking::bottom_nominations(account_id).expect("exists").nominations[0]
                    .owner,
                account_id_2
            );
            assert_eq!(
                ParachainStaking::bottom_nominations(account_id).expect("exists").nominations[0]
                    .amount,
                10
            );
            assert_eq!(ParachainStaking::bottom_nominations(account_id).unwrap().total, 10);
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
            assert_last_event!(MetaEvent::ParachainStaking(Event::NominationIncreased {
                nominator: account_id_2,
                candidate: account_id,
                amount: 5,
                in_top: false
            }));
            assert_eq!(
                ParachainStaking::bottom_nominations(account_id).expect("exists").nominations[0]
                    .owner,
                account_id_2
            );
            assert_eq!(
                ParachainStaking::bottom_nominations(account_id).expect("exists").nominations[0]
                    .amount,
                15
            );
            assert_eq!(ParachainStaking::bottom_nominations(account_id).unwrap().total, 15);
        });
}

#[test]
fn can_nominator_bond_extra_for_leaving_candidate() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 15)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 10)])
        .build()
        .execute_with(|| {
            assert_ok!(ParachainStaking::schedule_leave_candidates(Origin::signed(account_id), 1));
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
        });
}

#[test]
fn nominator_bond_extra_disallowed_when_revoke_scheduled() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 25)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 10)])
        .build()
        .execute_with(|| {
            assert_ok!(ParachainStaking::schedule_revoke_nomination(
                Origin::signed(account_id_2),
                account_id
            ));
            assert_noop!(
                ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5),
                <Error<Test>>::PendingNominationRevoke
            );
        });
}

#[test]
fn nominator_bond_extra_allowed_when_bond_decrease_scheduled() {
    let account_id = to_acc_id(1u64);
    let account_id_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![(account_id, 30), (account_id_2, 25)])
        .with_candidates(vec![(account_id, 30)])
        .with_nominations(vec![(account_id_2, account_id, 15)])
        .build()
        .execute_with(|| {
            assert_ok!(ParachainStaking::schedule_nominator_bond_less(
                Origin::signed(account_id_2),
                account_id,
                5,
            ));
            assert_ok!(ParachainStaking::bond_extra(Origin::signed(account_id_2), account_id, 5));
        });
}