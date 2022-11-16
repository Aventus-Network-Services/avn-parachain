use crate::*;

impl<T: Config> Pallet<T> {
    pub fn call_nominate(
        nominator: &T::AccountId,
        candidate: T::AccountId,
        amount: BalanceOf<T>,
        candidate_nomination_count: u32,
        nomination_count: u32,
    ) -> DispatchResultWithPostInfo {
        // check that caller can reserve the amount before any changes to storage
        ensure!(
            Self::get_nominator_stakable_free_balance(nominator) >= amount,
            Error::<T>::InsufficientBalance
        );

        let mut nominator_state = if let Some(mut state) = <NominatorState<T>>::get(nominator) {
            // The min amount for subsequent nominations on additional collators.
            ensure!(amount >= T::MinNominationPerCollator::get(), Error::<T>::NominationBelowMin);
            ensure!(
                nomination_count >= state.nominations.0.len() as u32,
                Error::<T>::TooLowNominationCountToNominate
            );
            ensure!(
                (state.nominations.0.len() as u32) < T::MaxNominationsPerNominator::get(),
                Error::<T>::ExceedMaxNominationsPerNominator
            );
            ensure!(
                state.add_nomination(Bond { owner: candidate.clone(), amount }),
                Error::<T>::AlreadyNominatedCandidate
            );
            state
        } else {
            // first nomination
            ensure!(
                amount >= <MinTotalNominatorStake<T>>::get(),
                Error::<T>::NominatorBondBelowMin
            );
            ensure!(!Self::is_candidate(nominator), Error::<T>::CandidateExists);
            Nominator::new(nominator.clone(), candidate.clone(), amount)
        };

        let mut state = <CandidateInfo<T>>::get(&candidate).ok_or(Error::<T>::CandidateDNE)?;
        ensure!(
            candidate_nomination_count >= state.nomination_count,
            Error::<T>::TooLowCandidateNominationCountToNominate
        );

        let (nominator_position, less_total_staked) =
            state.add_nomination::<T>(&candidate, Bond { owner: nominator.clone(), amount })?;

        // TODO: causes redundant free_balance check
        nominator_state.adjust_bond_lock::<T>(BondAdjust::Increase(amount))?;

        // only is_some if kicked the lowest bottom as a consequence of this new nomination
        let net_total_increase =
            if let Some(less) = less_total_staked { amount.saturating_sub(less) } else { amount };

        let new_total_locked = <Total<T>>::get().saturating_add(net_total_increase);
        <Total<T>>::put(new_total_locked);
        <CandidateInfo<T>>::insert(&candidate, state);
        <NominatorState<T>>::insert(nominator, nominator_state);

        Self::deposit_event(Event::Nomination {
            nominator: nominator.clone(),
            locked_amount: amount,
            candidate,
            nominator_position,
        });

        Ok(().into())
    }
}
