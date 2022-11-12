use crate::{ assert_event_emitted,
    mock::{
        roll_one_block, roll_to_era_begin, set_author, set_reward_pot,
        AccountId, ExtBuilder, Origin, ParachainStaking, Test,
        TestAccount, ErasPerGrowthPeriod, RewardPaymentDelay, System, get_default_block_per_era
    },
    BalanceOf, EraIndex, Event, GrowthInfo,
};
use frame_support::assert_ok;
use std::collections::HashMap;
use parity_scale_codec::{Encode, Decode};

const DEFAULT_POINTS: u32 = 5;

pub type Reward = u128;
pub type Stake = u128;

#[derive(Clone, Encode, Decode, Debug)]
pub struct GrowthData {
    pub reward: Reward,
    pub stake: Stake
}
impl GrowthData {
    pub fn new(reward: Reward, stake: Stake) -> Self {
        GrowthData {
            reward,
            stake
        }
    }
}

fn to_acc_id(id: u64) -> AccountId {
    return TestAccount::new(id).account_id()
}

fn roll_one_growth_period(current_era_index: EraIndex) -> u32 {
    roll_to_era_begin((current_era_index + ErasPerGrowthPeriod::get()).into());
    return ParachainStaking::era().current;
}

fn roll_one_era_and_try_paying_collators(current_era: EraIndex) -> EraIndex {
    // This will change era and trigger first collator payout (if any due)
    roll_to_era_begin((current_era + 1).into());
    // move one more block to finish paying out the second collator (if any due)
    roll_one_block();

    return ParachainStaking::era().current;
}

fn set_equal_points_for_collators(era: EraIndex, collator_1: AccountId, collator_2: AccountId) {
    set_author(era, collator_1, DEFAULT_POINTS);
    set_author(era, collator_2, DEFAULT_POINTS);
}

fn increase_collator_nomination(collator_1: AccountId, collator_2: AccountId, increase_amount: u128) {
    assert_ok!(ParachainStaking::candidate_bond_more(Origin::signed(collator_1), increase_amount));
    assert_ok!(ParachainStaking::candidate_bond_more(Origin::signed(collator_2), increase_amount));
}

fn get_expected_block_number(growth_index: u64) -> u64 {
    return get_default_block_per_era() as u64 * ErasPerGrowthPeriod::get() as u64 * growth_index;
}

fn increase_reward_pot_by(amount: u128) -> u128 {
    let current_balance = ParachainStaking::reward_pot();
    let new_balance = current_balance + amount;
    set_reward_pot(new_balance);
    return new_balance;
}

fn roll_foreward_and_pay_stakers(
    max_era: u32,
    collator_1: AccountId,
    collator_2: AccountId,
    collator1_stake: u128,
    collator2_stake: u128) -> HashMap<EraIndex, GrowthData>
{
    let mut raw_data: HashMap<EraIndex, GrowthData> = HashMap::from([(1, GrowthData::new(0u128, 30u128))]);
    let mut era_index = ParachainStaking::era().current;
    let mut total_stake = collator1_stake + collator2_stake;

    let initial_reward = 6u128;

    // SETUP: Run through era to generate realistic data. Change staked amount, reward for each era.
    for n in 1..=max_era - RewardPaymentDelay::get() {
        <frame_system::Pallet<Test>>::reset_events();

        if n == 1 {
            // No collator payouts on first era
            set_equal_points_for_collators(era_index, collator_1, collator_2);
            set_reward_pot(initial_reward);
            era_index = roll_one_era_and_try_paying_collators(era_index);

            assert_event_emitted!(Event::NewEra {
                starting_block: (get_default_block_per_era() as u64).into(),
                era: 2,
                selected_collators_number: 2,
                total_balance: total_stake,
            });

            raw_data.insert(era_index, GrowthData::new(initial_reward, total_stake as u128));
        }

        // Both collators will be paid from now on because we will be in era 3
        let reward_amount = (10 * n) as u128;
        let bond_increase_amount = (10 + n) as u128;

        increase_collator_nomination(collator_1, collator_2, bond_increase_amount);
        set_equal_points_for_collators(era_index, collator_1, collator_2);
        let new_reward_pot_amount = increase_reward_pot_by(reward_amount);

        era_index = roll_one_era_and_try_paying_collators(era_index);

        total_stake += bond_increase_amount * 2;
        assert_event_emitted!(Event::NewEra {
            starting_block: (get_default_block_per_era() as u64 * (era_index as u64 - 1u64)).into(),
            era: era_index,
            selected_collators_number: 2,
            total_balance: total_stake,
        });

        // When n == 1, reward is accrued because there is no payout to collators until the third era
        let expected_reward = new_reward_pot_amount / 2; //if n == 1 {(initial_reward + reward_amount) / 2} else {reward_amount / 2};

        assert_event_emitted!(Event::Rewarded {
            account: collator_1,
            rewards: expected_reward,
        });
        assert_event_emitted!(Event::Rewarded {
            account: collator_2,
            rewards: expected_reward,
        });

        raw_data.insert(era_index, GrowthData::new(new_reward_pot_amount, total_stake as u128));
    }

    return raw_data;
}

#[test]
fn initial_growth_state_is_ok() {
    ExtBuilder::default().build().execute_with(|| {
        let default_growth: GrowthInfo<AccountId, BalanceOf<Test>> = GrowthInfo::new(1u32);

        // Growth period starts from 0
        let growth_period_info = ParachainStaking::growth_period_info();
        assert_eq!(growth_period_info.start_era_index, 0u32);
        assert_eq!(growth_period_info.index, 0u32);

        // The first growth is empty
        let initial_growth = ParachainStaking::growth(0);
        assert_eq!(initial_growth.number_of_accumulations, default_growth.number_of_accumulations);
        assert_eq!(initial_growth.total_stake_accumulated, default_growth.total_stake_accumulated);
        assert_eq!(initial_growth.total_staker_reward, default_growth.total_staker_reward);
        assert_eq!(initial_growth.total_points, default_growth.total_points);
        assert_eq!(initial_growth.collator_scores, default_growth.collator_scores);
    });
}

#[test]
fn growth_period_indices_updated_correctly() {
    let collator_1 = to_acc_id(1u64);
    let collator_2 = to_acc_id(2u64);
    ExtBuilder::default()
        .with_balances(vec![
            (collator_1, 100),
            (collator_2, 100),
        ])
        .with_candidates(vec![
            (collator_1, 10),
            (collator_2, 10),
        ])
        .build()
        .execute_with(|| {
            let initial_growth_period_index = 0;
            let mut era_index = ParachainStaking::era().current;

            for n in 1..5 {
                set_author(era_index, collator_1, 5);
                set_author(era_index, collator_2, 5);
                era_index = roll_one_growth_period(era_index);

                let growth_period_info = ParachainStaking::growth_period_info();
                assert_eq!(
                    growth_period_info.start_era_index, era_index - RewardPaymentDelay::get(),
                    "Start era index for n={} does not match expected", n
                );
                assert_eq!(
                    growth_period_info.index, initial_growth_period_index + n,
                    "index for n={} does not match expected", n
                );
                assert_eq!(
                    System::block_number(), get_expected_block_number(growth_period_info.index.into()),
                    "Block number for n={} does not match expected", n
                );
            }
        });
}

mod growth_info_recorded_correctly {
    use super::*;

    #[test]
    fn for_one_single_period() {
        let collator_1 = to_acc_id(1u64);
        let collator_2 = to_acc_id(2u64);
        let collator1_stake = 20;
        let collator2_stake = 10;
        let reward_payment_delay = RewardPaymentDelay::get();
        ExtBuilder::default()
            .with_balances(vec![
                (collator_1, 10000),
                (collator_2, 10000),
            ])
            .with_candidates(vec![
                (collator_1, collator1_stake),
                (collator_2, collator2_stake),
            ])
            .build()
            .execute_with(|| {
                let num_era_to_roll_foreward = reward_payment_delay + 1;

                // Setup data by rolling forward and letting the system generate staking rewards. This is not "faked" data.
                let raw_era_data: HashMap<EraIndex, GrowthData> = roll_foreward_and_pay_stakers(
                    num_era_to_roll_foreward,
                    collator_1,
                    collator_2,
                    collator1_stake,
                    collator2_stake);

                // Verification: On era (RewardPaymentDelay + 1) we should have the first growth period created,
                // and since we only rolled that many times (num_era_to_roll_foreward = reward_payment_delay + 1),
                // we only expect a single entry growth info (no accumulation)

                // Check that we have the expected number of records added
                assert_eq!(ParachainStaking::growth_period_info().index, 1);

                let growth = ParachainStaking::growth(1);
                assert_eq!(growth.number_of_accumulations, 1);
                assert_eq!(growth.total_points, DEFAULT_POINTS * 2); // 2 collators

                // Check that total stake matches era 1's (3 - 2) stake because payouts are delayed by 2 eras
                assert_eq!(growth.total_stake_accumulated, raw_era_data.get(&1).unwrap().stake);

                // This is a bit tricky because `Reward` is not backdated, we pay whatever was in the reward pot at the time of payout
                // therefore it is a sum of reward for the current era we snapshoted this GrowthPeriod)
                let current_era_index = ParachainStaking::era().current;
                assert_eq!(growth.total_staker_reward, raw_era_data.get(&current_era_index).unwrap().reward);

                // Check that scores are recorded for both collators
                assert_eq!(growth.collator_scores.len(), 2usize);
            });
    }

    #[test]
    fn for_one_accumulated_period() {
        let collator_1 = to_acc_id(1u64);
        let collator_2 = to_acc_id(2u64);
        let collator1_stake = 20;
        let collator2_stake = 10;
        ExtBuilder::default()
            .with_balances(vec![
                (collator_1, 10000),
                (collator_2, 10000),
            ])
            .with_candidates(vec![
                (collator_1, collator1_stake),
                (collator_2, collator2_stake),
            ])
            .build()
            .execute_with(|| {
                let num_era_to_roll_foreward = RewardPaymentDelay::get() + ErasPerGrowthPeriod::get();

                // Setup data by rolling forward and letting the system generate staking rewards. This is not "faked" data.
                let raw_era_data: HashMap<EraIndex, GrowthData> = roll_foreward_and_pay_stakers(
                    num_era_to_roll_foreward,
                    collator_1,
                    collator_2,
                    collator1_stake,
                    collator2_stake);

                // Verification: On era (RewardPaymentDelay + 1) we should have the first growth period created,
                // and for the next 'ErasPerGrowthPeriod' we accumulate the data, thats why we rolled
                // (RewardPaymentDelay::get() + ErasPerGrowthPeriod::get()) eras.

                // Check that we have the expected number of records added
                let expected_number_of_growth_records = 1;
                assert_eq!(ParachainStaking::growth_period_info().index, expected_number_of_growth_records);

                let growth = ParachainStaking::growth(1);

                // Check that we accumulated the correct number of times
                assert_eq!(growth.number_of_accumulations, ErasPerGrowthPeriod::get());
                assert_eq!(growth.total_points, DEFAULT_POINTS * ErasPerGrowthPeriod::get() * 2); // 2 collators

                // Check that total stake matches eras 1 and 2 because payouts are delayed by 2 eras
                assert_eq!(growth.total_stake_accumulated,
                    raw_era_data.get(&1).unwrap().stake +
                    raw_era_data.get(&2).unwrap().stake
                );

                // Because `Reward` is not backdated, get the sum of reward for the current eras we snapshoted this GrowthPeriod
                assert_eq!(growth.total_staker_reward,
                    raw_era_data.get(&3).unwrap().reward +
                    raw_era_data.get(&4).unwrap().reward
                );
            });
    }

    #[test]
    fn for_multiple_periods() {
        let collator_1 = to_acc_id(1u64);
        let collator_2 = to_acc_id(2u64);
        let collator1_stake = 20;
        let collator2_stake = 10;
        let num_era_to_roll_foreward = 16;
        ExtBuilder::default()
            .with_balances(vec![
                (collator_1, 10000),
                (collator_2, 10000),
            ])
            .with_candidates(vec![
                (collator_1, collator1_stake),
                (collator_2, collator2_stake),
            ])
            .build()
            .execute_with(|| {
                assert_eq!(ErasPerGrowthPeriod::get(), 2, "This test will only work if ErasPerGrowthPeriod is set to 2");

                // Setup data by rolling forward and letting the system generate staking rewards. This is not "faked" data.
                let raw_era_data: HashMap<EraIndex, GrowthData> = roll_foreward_and_pay_stakers(
                    num_era_to_roll_foreward,
                    collator_1,
                    collator_2,
                    collator1_stake,
                    collator2_stake);

                let reward_payment_delay = RewardPaymentDelay::get();

                // VERIFY: check growth data automatically

                // Check that we have the expected number of records added
                let expected_number_of_growth_records = (raw_era_data.len() as u32 - reward_payment_delay) / reward_payment_delay;
                assert_eq!(ParachainStaking::growth_period_info().index, expected_number_of_growth_records);

                // payout era is always 'RewardPaymentDelay' (in this case 2 eras) behind the current era.
                let mut payout_era = (1, 2);
                // current era starts at the actual era this growth period has been created (3) and ends
                // at the last era accumulated by this growth period (4)
                let mut current_era = (3, 4);

                for n in 1..=expected_number_of_growth_records {
                    let growth = ParachainStaking::growth(n);

                    assert_eq!(growth.number_of_accumulations,
                        ErasPerGrowthPeriod::get(),
                        "total stake for n={} does not match expected value", n
                    );

                    assert_eq!(growth.total_points,
                        DEFAULT_POINTS * 2 * ErasPerGrowthPeriod::get(), // 2 collators
                        "total points for n={} does not match expected value", n
                    );

                    // This assumes that ErasPerGrowthPeriod = 2
                    assert_eq!(growth.total_stake_accumulated,
                        raw_era_data.get(&payout_era.0).unwrap().stake +
                        raw_era_data.get(&payout_era.1).unwrap().stake,
                        "total accumulation for n={} does not match expected value", n
                    );

                    // This is a bit tricky because `Reward` is not backdated, we pay whatever was in the reward pot at the time of payout
                    // therefore it is a sum of reward for the current era we snapshoted this GrowthPeriod)
                    assert_eq!(growth.total_staker_reward,
                        raw_era_data.get(&current_era.0).unwrap().reward +
                        raw_era_data.get(&current_era.1).unwrap().reward,
                        "total reward for n={} does not match expected value", n
                    );

                    // Check that scores are recorded for both collators
                    assert_eq!(growth.collator_scores.len(), 2usize);

                    payout_era = (payout_era.1 + 1, payout_era.1 + 2);
                    current_era = (current_era.1 + 1, current_era.1 + 2);
                }

            });
    }
}