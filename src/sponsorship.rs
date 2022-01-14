use crate::*;

#[derive(
    BorshStorageKey, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Debug,
)]
#[serde(crate = "near_sdk::serde")]
pub enum ProposalStatus {
    PENDING,
    REJECTED,
    ACCEPTED,
    RESCINDED,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ProposalSubmission<T> {
    pub description: String,
    pub tag: String,
    pub msg: Option<T>,
    pub duration: Option<U64>,
    pub deposit: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Proposal<T>
where
    T: BorshDeserialize + BorshSerialize,
{
    pub id: u64,
    pub description: String,
    pub tag: String,
    pub msg: Option<T>,
    pub author_id: AccountId,
    pub deposit: Balance,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub duration: Option<u64>,
    pub resolved_at: Option<u64>,
}

impl<T> Proposal<T>
where
    T: BorshDeserialize + BorshSerialize,
{
    pub fn is_expired(&self, now: u64) -> bool {
        match self.duration {
            Some(duration) => self.created_at + duration < now,
            None => false,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Sponsorship<T>
where
    T: BorshDeserialize + BorshSerialize,
{
    tags: UnorderedSet<String>,
    proposals: Vector<Proposal<T>>,
    proposal_duration: LazyOption<u64>,
    total_deposits: Balance,
    total_accepted_deposits: Balance,
}

impl<T> Sponsorship<T>
where
    T: BorshDeserialize + BorshSerialize,
{
    pub fn new<S>(storage_key_prefix: S, tags: Vec<String>, proposal_duration: Option<u64>) -> Self
    where
        S: IntoStorageKey,
    {
        let k = storage_key_prefix.into_storage_key();

        let mut tags_set = UnorderedSet::new(prefix_key(&k, b"t"));

        tags_set.extend(tags);

        Self {
            tags: tags_set,
            proposals: Vector::new(prefix_key(&k, b"p")),
            proposal_duration: LazyOption::new(prefix_key(&k, b"d"), proposal_duration.as_ref()),
            total_deposits: 0,
            total_accepted_deposits: 0,
        }
    }

    pub fn get_tags(&self) -> Vec<String> {
        self.tags.to_vec()
    }

    pub fn add_tags(&mut self, tags: Vec<String>) {
        self.tags.extend(tags)
    }

    pub fn remove_tags(&mut self, tags: Vec<String>) {
        for tag in tags {
            self.tags.remove(&tag);
        }
    }

    pub fn get_total_deposits(&self) -> U128 {
        self.total_deposits.into()
    }

    pub fn get_total_accepted_deposits(&self) -> U128 {
        self.total_accepted_deposits.into()
    }

    pub fn get_all(&self) -> Vec<Proposal<T>> {
        self.proposals.to_vec()
    }

    pub fn get_accepted(&self) -> Vec<Proposal<T>> {
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::ACCEPTED)
            .collect()
    }

    pub fn get_rejected(&self) -> Vec<Proposal<T>> {
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::REJECTED)
            .collect()
    }

    pub fn get_rescinded(&self) -> Vec<Proposal<T>> {
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::RESCINDED)
            .collect()
    }

    pub fn get_pending(&self) -> Vec<Proposal<T>> {
        let now = env::block_timestamp();
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::PENDING && !x.is_expired(now))
            .collect()
    }

    pub fn get_expired(&self) -> Vec<Proposal<T>> {
        let now = env::block_timestamp();
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::PENDING && x.is_expired(now))
            .collect()
    }

    pub fn get_proposal(&self, id: u64) -> Option<Proposal<T>> {
        self.proposals.get(id)
    }

    pub fn set_duration(&mut self, duration: Option<u64>) {
        if let Some(duration) = duration {
            self.proposal_duration.set(&duration);
        } else {
            self.proposal_duration.remove();
        }
    }

    pub fn get_duration(&self) -> Option<u64> {
        self.proposal_duration.get()
    }

    pub fn rescind(&mut self, id: u64) -> Proposal<T> {
        let proposal = self.proposals.get(id);
        require!(proposal.is_some(), "Proposal does not exist");
        let proposal = proposal.unwrap();
        require!(
            proposal.status == ProposalStatus::PENDING
                || proposal.status == ProposalStatus::REJECTED,
            "Proposal cannot be rescinded"
        );
        require!(
            proposal.author_id == env::predecessor_account_id(),
            "Proposal can only be rescinded by original author"
        );
        let now = env::block_timestamp();

        let resolved = Proposal {
            resolved_at: Some(now),
            status: ProposalStatus::RESCINDED,
            ..proposal
        };

        self.proposals.replace(id, &resolved);

        self.total_deposits -= proposal.deposit;

        let author_id = resolved.author_id.clone();
        log!(
            "Refunding rescinded deposit to {}: {}",
            &author_id,
            &resolved.deposit
        );
        Promise::new(author_id).transfer(resolved.deposit);

        resolved
    }

    fn resolve(&mut self, id: u64, accepted: bool) -> Proposal<T> {
        let proposal = self.proposals.get(id);
        require!(proposal.is_some(), "Proposal does not exist");
        let proposal = proposal.unwrap();
        require!(
            proposal.status == ProposalStatus::PENDING,
            "Proposal has already been resolved"
        );
        let now = env::block_timestamp();
        require!(!proposal.is_expired(now), "Proposal is expired");

        let resolved = Proposal {
            resolved_at: Some(now),
            status: if accepted {
                ProposalStatus::ACCEPTED
            } else {
                ProposalStatus::REJECTED
            },
            ..proposal
        };

        self.proposals.replace(id, &resolved);

        if accepted {
            self.total_accepted_deposits += proposal.deposit;
        }

        resolved
    }

    pub fn accept(&mut self, id: u64) -> Proposal<T> {
        self.resolve(id, true)
    }

    pub fn reject(&mut self, id: u64) -> Proposal<T> {
        self.resolve(id, false)
    }

    pub fn submit(&mut self, submission: ProposalSubmission<T>) -> Proposal<T> {
        let attached_deposit = env::attached_deposit();
        require!(attached_deposit >= 1, "Deposit required");

        let storage_usage_start = env::storage_usage();

        require!(self.tags.contains(&submission.tag), "Tag does not exist");

        let id = self.proposals.len();

        let duration = match (
            self.proposal_duration.get(),
            submission.duration.map(|x| x.into()),
        ) {
            (Some(contract_duration), Some(submission_duration)) => {
                Some(u64::min(contract_duration, submission_duration))
            }
            (Some(d), _) | (_, Some(d)) => Some(d),
            _ => None,
        };

        let submission_deposit = submission.deposit.into();

        let proposal = Proposal {
            id,
            author_id: env::predecessor_account_id(),
            description: submission.description,
            tag: submission.tag,
            msg: submission.msg,
            deposit: submission_deposit,
            created_at: env::block_timestamp(),
            duration,
            resolved_at: None,
            status: ProposalStatus::PENDING,
        };

        self.proposals.push(&proposal);

        let storage_usage_end = env::storage_usage();
        let storage_fee = Balance::from(storage_usage_end.saturating_sub(storage_usage_start))
            * env::storage_byte_cost();
        let total_required_deposit = storage_fee + submission_deposit;
        require!(
            attached_deposit >= total_required_deposit,
            format!(
                "Insufficient deposit. Required: {} yoctoNEAR Received: {} yoctoNEAR",
                &total_required_deposit, &attached_deposit
            )
        );

        let refund = attached_deposit - total_required_deposit;

        log!("Storage fee: {} Refund: {}", &storage_fee, &refund);

        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }

        self.total_deposits += proposal.deposit;

        proposal
    }
}

pub trait Sponsorable<T>
where
    T: BorshDeserialize + BorshSerialize,
{
    fn spo_get_tags(&self) -> Vec<String>;
    fn spo_add_tags(&mut self, tags: Vec<String>);
    fn spo_remove_tags(&mut self, tags: Vec<String>);
    fn spo_get_total_deposits(&self) -> U128;
    fn spo_get_total_accepted_deposits(&self) -> U128;
    fn spo_get_all_proposals(&self) -> Vec<Proposal<T>>;
    fn spo_get_pending_proposals(&self) -> Vec<Proposal<T>>;
    fn spo_get_accepted_proposals(&self) -> Vec<Proposal<T>>;
    fn spo_get_rejected_proposals(&self) -> Vec<Proposal<T>>;
    fn spo_get_rescinded_proposals(&self) -> Vec<Proposal<T>>;
    fn spo_get_expired_proposals(&self) -> Vec<Proposal<T>>;
    fn spo_get_proposal(&self, id: U64) -> Option<Proposal<T>>;
    fn spo_get_duration(&self) -> Option<U64>;
    fn spo_set_duration(&mut self, duration: Option<U64>);
    fn spo_submit(&mut self, submission: ProposalSubmission<T>) -> Proposal<T>;
    fn spo_accept(&mut self, id: U64) -> Proposal<T>;
    fn spo_reject(&mut self, id: U64) -> Proposal<T>;
    fn spo_rescind(&mut self, id: U64) -> Proposal<T>;
}

#[macro_export]
macro_rules! impl_sponsorship {
    ($contract: ident, $sponsorship: ident, $sponsorship_type: ident, $ownership: ident $(, $on_status_change: ident)? $(,)?) => {
        #[near_bindgen]
        impl Sponsorable<$sponsorship_type> for $contract {
            fn spo_get_tags(&self) -> Vec<String> {
                self.$sponsorship.get_tags()
            }

            #[payable]
            fn spo_add_tags(&mut self, tags: Vec<String>) {
                assert_one_yocto();
                self.$ownership.assert_owner();
                self.$sponsorship.add_tags(tags)
            }

            #[payable]
            fn spo_remove_tags(&mut self, tags: Vec<String>) {
                assert_one_yocto();
                self.$ownership.assert_owner();
                self.$sponsorship.remove_tags(tags)
            }

            fn spo_get_total_deposits(&self) -> U128 {
                self.$sponsorship.get_total_deposits()
            }

            fn spo_get_total_accepted_deposits(&self) -> U128 {
                self.$sponsorship.get_total_accepted_deposits()
            }

            fn spo_get_all_proposals(&self) -> Vec<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_all()
            }

            fn spo_get_pending_proposals(&self) -> Vec<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_pending()
            }

            fn spo_get_accepted_proposals(&self) -> Vec<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_accepted()
            }

            fn spo_get_rejected_proposals(&self) -> Vec<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_rejected()
            }

            fn spo_get_rescinded_proposals(&self) -> Vec<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_rescinded()
            }

            fn spo_get_expired_proposals(&self) -> Vec<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_expired()
            }

            fn spo_get_proposal(&self, id: U64) -> Option<Proposal<$sponsorship_type>> {
                self.$sponsorship.get_proposal(id.into())
            }

            fn spo_get_duration(&self) -> Option<U64> {
                self.$sponsorship.get_duration().map(|x| x.into())
            }

            #[payable]
            fn spo_set_duration(&mut self, duration: Option<U64>) {
                assert_one_yocto();
                self.$sponsorship.set_duration(duration.map(|x| x.into()))
            }

            #[payable]
            fn spo_submit(&mut self, submission: ProposalSubmission<$sponsorship_type>) -> Proposal<$sponsorship_type> {
                // submit manages its own deposit requirements
                let proposal = self.$sponsorship.submit(submission);
                $(self.$on_status_change(&proposal);)?
                proposal
            }

            #[payable]
            fn spo_accept(&mut self, id: U64) -> Proposal<$sponsorship_type> {
                assert_one_yocto();
                self.$ownership.assert_owner();
                let proposal = self.$sponsorship.accept(id.into());
                $(self.$on_status_change(&proposal);)?
                proposal
            }

            #[payable]
            fn spo_reject(&mut self, id: U64) -> Proposal<$sponsorship_type> {
                assert_one_yocto();
                self.$ownership.assert_owner();
                let proposal = self.$sponsorship.reject(id.into());
                $(self.$on_status_change(&proposal);)?
                proposal
            }

            #[payable]
            fn spo_rescind(&mut self, id: U64) -> Proposal<$sponsorship_type> {
                assert_one_yocto();
                let proposal = self.$sponsorship.rescind(id.into());
                $(self.$on_status_change(&proposal);)?
                proposal
            }
        }
    };
}
