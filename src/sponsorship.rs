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
pub struct ProposalSubmission {
    pub description: String,
    pub tag: String,
    pub msg: Option<String>,
    pub duration: Option<u64>,
    pub deposit: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Proposal {
    pub id: u64,
    pub description: String,
    pub tag: String,
    pub msg: Option<String>,
    pub author_id: AccountId,
    pub deposit: Balance,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub duration: Option<u64>,
    pub resolved_at: Option<u64>,
}

impl Proposal {
    pub fn is_expired(&self, now: u64) -> bool {
        match self.duration {
            Some(duration) => self.created_at + duration < now,
            None => false,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Sponsorship {
    tags: UnorderedSet<String>,
    proposals: Vector<Proposal>,
    proposal_duration: LazyOption<u64>,
    total_deposits: Balance,
    total_accepted_deposits: Balance,
}

impl Sponsorship {
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

    pub fn get_total_deposits(&self) -> Balance {
        self.total_deposits
    }

    pub fn get_total_accepted_deposits(&self) -> Balance {
        self.total_accepted_deposits
    }

    pub fn get_all(&self) -> Vec<Proposal> {
        self.proposals.to_vec()
    }

    pub fn get_accepted(&self) -> Vec<Proposal> {
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::ACCEPTED)
            .collect()
    }

    pub fn get_rejected(&self) -> Vec<Proposal> {
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::REJECTED)
            .collect()
    }

    pub fn get_rescinded(&self) -> Vec<Proposal> {
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::RESCINDED)
            .collect()
    }

    pub fn get_pending(&self) -> Vec<Proposal> {
        let now = env::block_timestamp();
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::PENDING && !x.is_expired(now))
            .collect()
    }

    pub fn get_expired(&self) -> Vec<Proposal> {
        let now = env::block_timestamp();
        self.proposals
            .iter()
            .filter(|x| x.status == ProposalStatus::PENDING && x.is_expired(now))
            .collect()
    }

    pub fn get_proposal(&self, id: u64) -> Option<Proposal> {
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

    pub fn rescind(&mut self, id: u64) -> Proposal {
        let proposal = self.proposals.get(id);
        require!(proposal.is_some(), "Proposal does not exist");
        let proposal = proposal.unwrap();
        require!(
            proposal.status == ProposalStatus::PENDING,
            "Proposal has already been resolved"
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

    fn resolve(&mut self, id: u64, accepted: bool) -> Proposal {
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

    pub fn accept(&mut self, id: u64) -> Proposal {
        self.resolve(id, true)
    }

    pub fn reject(&mut self, id: u64) -> Proposal {
        self.resolve(id, false)
    }

    pub fn submit(&mut self, submission: ProposalSubmission) -> Proposal {
        let attached_deposit = env::attached_deposit();
        require!(attached_deposit >= 1, "Deposit required");

        let storage_usage_start = env::storage_usage();

        require!(self.tags.contains(&submission.tag), "Tag does not exist");

        let id = self.proposals.len();

        let duration = match (self.proposal_duration.get(), submission.duration) {
            (Some(contract_duration), Some(submission_duration)) => {
                Some(u64::min(contract_duration, submission_duration))
            }
            (Some(d), _) | (_, Some(d)) => Some(d),
            _ => None,
        };

        let proposal = Proposal {
            id,
            author_id: env::predecessor_account_id(),
            description: submission.description,
            tag: submission.tag,
            msg: submission.msg,
            deposit: submission.deposit,
            created_at: env::block_timestamp(),
            duration,
            resolved_at: None,
            status: ProposalStatus::PENDING,
        };

        self.proposals.push(&proposal);

        let storage_usage_end = env::storage_usage();
        let storage_fee = Balance::from(storage_usage_end.saturating_sub(storage_usage_start))
            * env::storage_byte_cost();
        let total_required_deposit = storage_fee + submission.deposit;
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

pub trait Sponsorable {
    fn spo_get_tags(&self) -> Vec<String>;
    fn spo_add_tags(&mut self, tags: Vec<String>);
    fn spo_remove_tags(&mut self, tags: Vec<String>);
    fn spo_get_total_deposits(&self) -> Balance;
    fn spo_get_total_accepted_deposits(&self) -> Balance;
    fn spo_get_all_proposals(&self) -> Vec<Proposal>;
    fn spo_get_pending_proposals(&self) -> Vec<Proposal>;
    fn spo_get_accepted_proposals(&self) -> Vec<Proposal>;
    fn spo_get_rejected_proposals(&self) -> Vec<Proposal>;
    fn spo_get_rescinded_proposals(&self) -> Vec<Proposal>;
    fn spo_get_expired_proposals(&self) -> Vec<Proposal>;
    fn spo_get_proposal(&self, id: u64) -> Option<Proposal>;
    fn spo_get_duration(&self) -> Option<u64>;
    fn spo_set_duration(&mut self, duration: Option<u64>);
    fn spo_submit(&mut self, submission: ProposalSubmission) -> Proposal;
    fn spo_accept(&mut self, id: u64) -> Proposal;
    fn spo_reject(&mut self, id: u64) -> Proposal;
    fn spo_rescind(&mut self, id: u64) -> Proposal;
}

#[macro_export]
macro_rules! impl_sponsorship {
    ($contract: ident, $sponsorship: ident, $ownership: ident $(, $on_status_change: ident)? $(,)?) => {
        #[near_bindgen]
        impl Sponsorable for $contract {
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

            fn spo_get_total_deposits(&self) -> Balance {
                self.$sponsorship.get_total_deposits()
            }

            fn spo_get_total_accepted_deposits(&self) -> Balance {
                self.$sponsorship.get_total_accepted_deposits()
            }

            fn spo_get_all_proposals(&self) -> Vec<Proposal> {
                self.$sponsorship.get_all()
            }

            fn spo_get_pending_proposals(&self) -> Vec<Proposal> {
                self.$sponsorship.get_pending()
            }

            fn spo_get_accepted_proposals(&self) -> Vec<Proposal> {
                self.$sponsorship.get_accepted()
            }

            fn spo_get_rejected_proposals(&self) -> Vec<Proposal> {
                self.$sponsorship.get_rejected()
            }

            fn spo_get_rescinded_proposals(&self) -> Vec<Proposal> {
                self.$sponsorship.get_rescinded()
            }

            fn spo_get_expired_proposals(&self) -> Vec<Proposal> {
                self.$sponsorship.get_expired()
            }

            fn spo_get_proposal(&self, id: u64) -> Option<Proposal> {
                self.$sponsorship.get_proposal(id)
            }

            fn spo_get_duration(&self) -> Option<u64> {
                self.$sponsorship.get_duration()
            }

            #[payable]
            fn spo_set_duration(&mut self, duration: Option<u64>) {
                assert_one_yocto();
                self.$sponsorship.set_duration(duration)
            }

            fn spo_submit(&mut self, submission: ProposalSubmission) -> Proposal {
                // submit manages its own deposit requirements
                let proposal = self.$sponsorship.submit(submission);
                $(self.$on_status_change(&proposal);)?
                proposal
            }

            #[payable]
            fn spo_accept(&mut self, id: u64) -> Proposal {
                assert_one_yocto();
                self.$ownership.assert_owner();
                let proposal = self.$sponsorship.accept(id);
                $(self.$on_status_change(&proposal);)?
                proposal
            }

            #[payable]
            fn spo_reject(&mut self, id: u64) -> Proposal {
                assert_one_yocto();
                self.$ownership.assert_owner();
                let proposal = self.$sponsorship.reject(id);
                $(self.$on_status_change(&proposal);)?
                proposal
            }

            #[payable]
            fn spo_rescind(&mut self, id: u64) -> Proposal {
                assert_one_yocto();
                let proposal = self.$sponsorship.rescind(id);
                $(self.$on_status_change(&proposal);)?
                proposal
            }
        }
    };
}
