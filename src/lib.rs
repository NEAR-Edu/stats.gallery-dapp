use near_sdk::{
    borsh::{self, *},
    collections::*,
    serde::{self, *},
    *,
};

mod utils;
use utils::*;

mod ownership;
use ownership::*;

mod sponsorship;
use sponsorship::*;

mod contract;
pub use contract::*;

#[cfg(test)]
mod tests {
    use crate::*;
    use near_sdk::{test_utils::*, testing_env};

    fn contract_account() -> AccountId {
        "contract".parse::<AccountId>().unwrap()
    }

    fn owner_account() -> AccountId {
        "owner".parse::<AccountId>().unwrap()
    }

    fn proposed_owner_account() -> AccountId {
        "proposed_owner".parse::<AccountId>().unwrap()
    }

    fn sponsorship_tags() -> Vec<String> {
        vec!["tag_one", "tag_two", "tag_three"]
            .iter()
            .map(|x| x.to_string())
            .collect()
    }

    const PROPOSAL_DURATION: u64 = 1_000_000_000 * 60 * 60 * 24 * 7;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(contract_account())
            .account_balance(14500000000000000000000000)
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn create_instance() -> StatsGallery {
        StatsGallery::new(owner_account(), sponsorship_tags(), PROPOSAL_DURATION)
    }

    #[test]
    fn instantiate() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let c = create_instance();
        assert_eq!(
            owner_account(),
            c.own_get_owner().unwrap(),
            "Owner should be owner account after instantiation",
        );
        assert_eq!(
            None,
            c.own_get_proposed_owner(),
            "There should be no proposed owner after instantiation",
        );
        assert_eq!(
            0,
            c.spo_get_all_proposals().len(),
            "There should be no sponsorship proposals after instantiation",
        );
        assert_eq!(
            sponsorship_tags(),
            c.spo_get_tags(),
            "Sponsorship tags should be correctly initialized",
        );
        assert_eq!(
            Some(PROPOSAL_DURATION),
            c.spo_get_duration(),
            "Proposal duration should be properly initialized",
        );
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn renounce_owner_no_deposit() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_renounce_owner();
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn renounce_owner_not_owner() {
        let mut context = get_context(accounts(1));
        context.attached_deposit(1);
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_renounce_owner();
    }

    #[test]
    fn renounce_owner() {
        let mut context = get_context(owner_account());
        context.attached_deposit(1u128.into());
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_renounce_owner();
        assert_eq!(
            None,
            c.own_get_owner(),
            "There should be no owner after renounce"
        );
        assert_eq!(
            None,
            c.own_get_proposed_owner(),
            "There should be no proposed owner after renounce"
        );
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn propose_owner_no_deposit() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_propose_owner(Some(proposed_owner_account()));
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn propose_owner_not_owner() {
        let mut context = get_context(accounts(1));
        context.attached_deposit(1);
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_propose_owner(Some(proposed_owner_account()));
    }

    #[test]
    fn propose_owner() {
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_propose_owner(Some(proposed_owner_account()));
        assert_eq!(
            owner_account(),
            c.own_get_owner().unwrap(),
            "Owner should not change after proposing new owner"
        );
        assert_eq!(
            proposed_owner_account(),
            c.own_get_proposed_owner().unwrap(),
            "Proposed owner should update after proposing new owner"
        );
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn accept_owner_no_deposit() {
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_propose_owner(Some(proposed_owner_account()));

        let context = get_context(proposed_owner_account());
        testing_env!(context.build());
        c.own_accept_owner();
    }

    #[test]
    #[should_panic(expected = "Proposed owner only")]
    fn accept_owner_not_proposed() {
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_propose_owner(Some(proposed_owner_account()));

        let mut context = get_context(accounts(2));
        context.attached_deposit(1);
        testing_env!(context.build());
        c.own_accept_owner();
    }

    #[test]
    fn accept_owner() {
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());
        let mut c = create_instance();
        c.own_propose_owner(Some(proposed_owner_account()));

        let mut context = get_context(proposed_owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());
        c.own_accept_owner();
        assert_eq!(
            proposed_owner_account(),
            c.own_get_owner().unwrap(),
            "Owner should be proposed owner after accepting change",
        );
        assert_eq!(
            None,
            c.own_get_proposed_owner(),
            "There should be no proposed owner after accepting proposal",
        );
    }

    #[test]
    fn submit_proposal() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));
        context.attached_deposit(proposal_deposit + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });

        assert_eq!(0, proposal.id, "Should be first proposal",);
        assert_eq!(
            "This is a sponsorship proposal".to_string(),
            proposal.description,
            "Should be first proposal",
        );
        assert_eq!(
            proposal_deposit, proposal.deposit,
            "Should have attached correct deposit",
        );
        assert_eq!(
            true,
            c.spo_get_all_proposals().contains(&proposal),
            "Should be a member of all proposals",
        );
        assert_eq!(
            true,
            c.spo_get_pending_proposals().contains(&proposal),
            "Should be a member of pending proposals",
        );
        assert_eq!(
            ProposalStatus::PENDING,
            proposal.status,
            "Proposal status should be pending after submission",
        );
        assert_eq!(
            accounts(1),
            proposal.author_id,
            "Proposal author account ID should be that of submitter",
        );
        assert_eq!(
            proposal,
            c.spo_get_proposal(proposal.id).unwrap(),
            "Proposal should be indexed by ID",
        );
    }

    #[test]
    #[should_panic(expected = "Deposit required")]
    fn submit_proposal_no_deposit() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));

        testing_env!(context.build());
        c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit.")]
    fn submit_proposal_insufficient_deposit() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));

        // Missing deposit for storage
        context.attached_deposit(proposal_deposit /* + 10u128.pow(22) */);

        testing_env!(context.build());
        c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });
    }

    #[test]
    fn rescind_proposal() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));
        context.attached_deposit(proposal_deposit + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });

        let mut context = get_context(accounts(1));
        context.attached_deposit(1);
        testing_env!(context.build());

        let balance_before_rescind = env::account_balance();

        let proposal = c.spo_rescind(proposal.id);

        let balance_after_rescind = env::account_balance();

        assert_eq!(
            proposal.deposit,
            balance_before_rescind - balance_after_rescind,
            "Deposit should be returned",
        );
        assert_eq!(
            true,
            c.spo_get_all_proposals().contains(&proposal),
            "Should be a member of all proposals",
        );
        assert_eq!(
            true,
            c.spo_get_rescinded_proposals().contains(&proposal),
            "Should be a member of rescinded proposals",
        );
        assert_eq!(
            ProposalStatus::RESCINDED,
            proposal.status,
            "Proposal status should be rescinded",
        );
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn rescind_proposal_no_deposit() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));
        context.attached_deposit(proposal_deposit + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });

        let context = get_context(accounts(1));
        // context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id);
    }

    #[test]
    #[should_panic(expected = "Proposal can only be rescinded by original author")]
    fn rescind_proposal_not_author() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));
        context.attached_deposit(proposal_deposit + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });

        let mut context = get_context(accounts(2));
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id);
    }

    #[test]
    #[should_panic(expected = "Proposal is expired")]
    fn rescind_proposal_expired() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));
        context
            .attached_deposit(proposal_deposit + 10u128.pow(22))
            .block_timestamp(1_000_000_000);
        testing_env!(context.build());
        let proposal = c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });

        let mut context = get_context(accounts(1));
        context
            .attached_deposit(1)
            .block_timestamp(1_000_000_000 + PROPOSAL_DURATION + 1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id);
    }

    #[test]
    #[should_panic(expected = "Proposal has already been resolved")]
    fn rescind_proposal_already_resolved() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let proposal_deposit = Balance::from(10u128.pow(24));
        context
            .attached_deposit(proposal_deposit + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: proposal_deposit,
            duration: None,
            msg: None,
            tag: sponsorship_tags()[0].to_string(),
        });

        let mut context = get_context(accounts(1));
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id);
        // Cannot rescind twice
        c.spo_rescind(proposal.id);
    }
}