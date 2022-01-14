use near_sdk::{
    borsh::{self, *},
    collections::*,
    json_types::*,
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
        vec![contract::TAG_BADGE_CREATE, contract::TAG_BADGE_EXTEND]
            .iter()
            .map(|x| x.to_string())
            .collect()
    }

    const ONE_DAY: u64 = 1_000_000_000 * 60 * 60 * 24; // nanoseconds
    const BADGE_MAX_ACTIVE_DURATION: u64 = ONE_DAY * 180;
    const PROPOSAL_DURATION: u64 = ONE_DAY * 7;

    const ONE_NEAR: u128 = u128::pow(10, 24);
    const BADGE_RATE_PER_DAY: u128 = ONE_NEAR / 10; // 0.1 NEAR
    const BADGE_MIN_CREATION_DEPOSIT: u128 = ONE_NEAR * 3 / 2; // 1.5 NEAR

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(contract_account())
            .account_balance(15 * ONE_NEAR)
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn create_instance() -> StatsGallery {
        StatsGallery::new(
            owner_account(),
            PROPOSAL_DURATION.into(),
            BADGE_RATE_PER_DAY.into(),
            BADGE_MAX_ACTIVE_DURATION.into(),
            BADGE_MIN_CREATION_DEPOSIT.into(),
        )
    }

    fn calculate_deposit(action: &BadgeAction) -> Balance {
        match action {
            BadgeAction::Create(create_request) => Balance::max(
                BADGE_MIN_CREATION_DEPOSIT,
                Balance::from(billable_days_in_duration(create_request.duration))
                    * BADGE_RATE_PER_DAY,
            ),
            BadgeAction::Extend(extend_request) => {
                Balance::from(billable_days_in_duration(extend_request.duration))
                    * BADGE_RATE_PER_DAY
            }
        }
    }

    fn badge_create() -> BadgeCreate {
        BadgeCreate {
            id: String::from("my-badge-01"),
            group_id: String::from("my-badge"),
            name: String::from("Cool Badge"),
            description: String::from("This is a badge you earn from doing cool stuff"),
            duration: ONE_DAY * 45,
            start_at: None,
        }
    }

    fn badge_extend() -> BadgeExtend {
        BadgeExtend {
            id: String::from("my-badge-01"),
            duration: ONE_DAY * 12,
        }
    }

    fn proposal_submission(action: BadgeAction, tag: String) -> ProposalSubmission<BadgeAction> {
        ProposalSubmission {
            description: "This is a sponsorship proposal".to_string(),
            deposit: U128(calculate_deposit(&action)),
            duration: Some(U64(ONE_DAY * 45)),
            msg: Some(action),
            tag,
        }
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
            c.spo_get_duration().map(|x| x.into()),
            "Proposal duration should be properly initialized",
        );
        assert_eq!(
            BADGE_MAX_ACTIVE_DURATION,
            u64::from(c.get_badge_max_active_duration()),
            "Badge max active duration should be properly initialized",
        );
        assert_eq!(
            BADGE_MIN_CREATION_DEPOSIT,
            u128::from(c.get_badge_min_creation_deposit()),
            "Badge min creation deposit should be properly initialized",
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
    fn serialize_actions() {
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );

        log!("{}", serde_json::to_string(&submission).unwrap());
    }

    #[test]
    fn submit_proposal() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        let submission_deposit: u128 = submission.deposit.into();
        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        assert_eq!(0, proposal.id, "Should be first proposal",);
        assert_eq!(
            "This is a sponsorship proposal".to_string(),
            proposal.description,
            "Should be first proposal",
        );
        assert_eq!(
            submission_deposit, proposal.deposit,
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
            c.spo_get_proposal(proposal.id.into()).unwrap(),
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
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );

        testing_env!(context.build());
        c.spo_submit(submission);
    }

    #[test]
    #[should_panic(expected = "Insufficient deposit.")]
    fn submit_proposal_insufficient_deposit() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        // Missing deposit for storage
        context.attached_deposit(u128::from(submission.deposit) /* + 10u128.pow(22) */);

        testing_env!(context.build());
        c.spo_submit(submission);
    }

    #[test]
    fn rescind_proposal() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        let mut context = get_context(accounts(1));
        context.attached_deposit(1);
        testing_env!(context.build());

        let balance_before_rescind = env::account_balance();

        let proposal = c.spo_rescind(proposal.id.into());

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
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        let context = get_context(accounts(1));
        // context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id.into());
    }

    #[test]
    #[should_panic(expected = "Proposal can only be rescinded by original author")]
    fn rescind_proposal_not_author() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        let mut context = get_context(accounts(2));
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id.into());
    }

    #[test]
    fn rescind_proposal_expired() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context
            .attached_deposit(u128::from(submission.deposit) + 10u128.pow(22))
            .block_timestamp(1_000_000_000);

        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        let mut context = get_context(accounts(1));
        context
            .attached_deposit(1)
            .block_timestamp(1_000_000_000 + PROPOSAL_DURATION + 1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id.into());
    }

    #[test]
    #[should_panic(expected = "Proposal has already been resolved")]
    fn rescind_proposal_already_resolved() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        let mut context = get_context(accounts(1));
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_rescind(proposal.id.into());
        // Cannot rescind twice
        c.spo_rescind(proposal.id.into());
    }

    #[test]
    fn create_badge() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        // Submit badge creation request
        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let proposal = c.spo_submit(submission);

        // Accept badge creation request
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_accept(proposal.id.into());

        require!(c.get_badges().len() == 1, "There should be one badge",);

        let expected = badge_create();
        let actual = c.get_badge(expected.id.clone());
        require!(
            actual.is_some(),
            "Badge is activated and accessible by its ID",
        );

        let actual = actual.unwrap();

        require!(actual.id == expected.id, "IDs match",);
        require!(
            actual.name == expected.name
                && actual.description == expected.description
                && actual.duration.unwrap() == expected.duration
                && actual.group_id == expected.group_id,
            "Metadata match",
        );
    }

    #[test]
    #[should_panic(expected = "tag mismatch")]
    fn create_badge_tag_mismatch() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        // Submit badge creation request
        let mut context = get_context(accounts(1));
        let submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_EXTEND.to_string(),
        );
        context.attached_deposit(u128::from(submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        c.spo_submit(submission);
    }

    #[test]
    fn extend_badge() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        // Submit badge creation request
        let mut context = get_context(accounts(1));
        let create_submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(create_submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let create_proposal = c.spo_submit(create_submission);

        // Accept badge creation request
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_accept(create_proposal.id.into());

        // Submit badge extension request
        let mut context = get_context(accounts(1));
        let extend_submission = proposal_submission(
            BadgeAction::Extend(badge_extend()),
            TAG_BADGE_EXTEND.to_string(),
        );

        context.attached_deposit(u128::from(extend_submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let extend_proposal = c.spo_submit(extend_submission);

        // Accept badge extension request
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_accept(extend_proposal.id.into());

        let expected_create = badge_create();
        let expected = badge_extend();
        let actual = c.get_badge(expected.id.clone());

        require!(actual.is_some(), "Badge exists after extend",);

        let actual = actual.unwrap();

        require!(expected.id == actual.id, "IDs match",);
        require!(
            actual.duration.unwrap() == expected.duration + expected_create.duration,
            "Duration after extend should be sum of original and extend request"
        );
    }

    #[test]
    #[should_panic(expected = "Exceeded maximum active duration")]
    fn extend_badge_exceeds_max_duration() {
        let context = get_context(owner_account());
        testing_env!(context.build());
        let mut c = create_instance();

        // Submit badge creation request
        let mut context = get_context(accounts(1));
        let create_submission = proposal_submission(
            BadgeAction::Create(badge_create()),
            TAG_BADGE_CREATE.to_string(),
        );
        context.attached_deposit(u128::from(create_submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        let create_proposal = c.spo_submit(create_submission);

        // Accept badge creation request
        let mut context = get_context(owner_account());
        context.attached_deposit(1);
        testing_env!(context.build());

        c.spo_accept(create_proposal.id.into());

        // Submit badge extension request
        let mut context = get_context(accounts(1));
        let original = BadgeExtend {
            duration: BADGE_MAX_ACTIVE_DURATION - badge_create().duration + 1, // should exceed max duration by 1
            ..badge_extend()
        };
        let extend_submission =
            proposal_submission(BadgeAction::Extend(original), TAG_BADGE_EXTEND.to_string());

        context.attached_deposit(u128::from(extend_submission.deposit) + 10u128.pow(22));
        testing_env!(context.build());
        c.spo_submit(extend_submission);
    }
}
