use crate::impl_ownership;
use crate::*;

pub const TAG_BADGE_CREATE: &'static str = "badge_create";
pub const TAG_BADGE_EXTEND: &'static str = "badge_extend";

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    OWNERSHIP,
    SPONSORSHIP,
    BADGES,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Badge {
    pub id: String,
    pub group_id: String,
    pub name: String,
    pub description: String,
    pub is_enabled: bool,
    pub created_at: u64,
    pub start_at: u64,
    pub duration: Option<u64>,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum BadgeAction {
    Create(BadgeCreate),
    Extend(BadgeExtend),
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct BadgeCreate {
    pub id: String,
    pub group_id: String,
    pub name: String,
    pub description: String,
    pub start_at: Option<u64>,
    pub duration: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct BadgeExtend {
    pub id: String,
    pub duration: u64,
}

impl Badge {
    pub fn is_expired(&self, now: u64) -> bool {
        match self.duration {
            Some(duration) => self.created_at + duration < now,
            _ => false, // No duration = never expires
        }
    }
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct StatsGallery {
    ownership: Ownership,
    sponsorship: Sponsorship<BadgeAction>,
    badges: UnorderedMap<String, Badge>,
    badge_rate_per_day: Balance,
    badge_max_active_duration: u64,
    badge_min_creation_deposit: Balance,
}

const DAY: u64 = 1_000_000_000 * 60 * 60 * 24;

// Basically unstable_div_ceil
pub fn billable_days_in_duration(duration: u64) -> u64 {
    duration / DAY + if duration % DAY > 0 { 1 } else { 0 }
}

macro_rules! extract_msg {
    ($proposal: ident, $enum: ident, $variant: ident) => {
        match &$proposal.msg {
            Some($enum::$variant(value)) => value,
            Some(..) => env::panic_str("tag mismatch"),
            _ => env::panic_str("msg value required"),
        }
    };
}

#[near_bindgen]
impl StatsGallery {
    #[init]
    pub fn new(
        owner_id: AccountId,
        proposal_duration: u64,
        badge_rate_per_day: Balance,
        badge_max_active_duration: u64,
        badge_min_creation_deposit: Balance,
    ) -> Self {
        Self {
            ownership: Ownership::new(StorageKey::OWNERSHIP, owner_id),
            sponsorship: Sponsorship::new(
                StorageKey::SPONSORSHIP,
                vec![TAG_BADGE_CREATE.to_string(), TAG_BADGE_EXTEND.to_string()],
                Some(proposal_duration),
            ),
            badges: UnorderedMap::new(StorageKey::BADGES),
            badge_rate_per_day,
            badge_max_active_duration,
            badge_min_creation_deposit,
        }
    }

    pub fn get_badges(&self) -> Vec<Badge> {
        let now = env::block_timestamp();

        self.badges
            .values()
            .filter(|b| b.is_enabled && !b.is_expired(now))
            .collect()
    }

    pub fn get_badge(&self, badge_id: String) -> Option<Badge> {
        self.badges.get(&badge_id)
    }

    #[payable]
    pub fn set_badge_is_enabled(&mut self, badge_id: String, is_enabled: bool) -> Badge {
        assert_one_yocto();
        self.ownership.assert_owner();

        let badge = self
            .badges
            .get(&badge_id)
            .unwrap_or_else(|| env::panic_str("Badge does not exist"));

        let new_badge = Badge {
            is_enabled,
            ..badge
        };

        self.badges.insert(&badge_id, &new_badge);

        new_badge
    }

    #[payable]
    pub fn insert_badge(&mut self, badge: Badge) {
        assert_one_yocto();
        self.ownership.assert_owner();

        self.badges.insert(&badge.id, &badge);
    }

    #[payable]
    pub fn remove_badge(&mut self, badge_id: &String) {
        assert_one_yocto();
        self.ownership.assert_owner();

        self.badges.remove(&badge_id);
    }

    pub fn get_badge_rate_per_day(&self) -> Balance {
        self.badge_rate_per_day
    }

    #[payable]
    pub fn set_badge_rate_per_day(&mut self, badge_rate_per_day: Balance) {
        assert_one_yocto();
        self.ownership.assert_owner();
        require!(badge_rate_per_day > 0, "Badge rate must be greater than 0");

        self.badge_rate_per_day = badge_rate_per_day;
    }

    pub fn get_badge_max_active_duration(&self) -> u64 {
        self.badge_max_active_duration
    }

    #[payable]
    pub fn set_badge_max_active_duration(&mut self, badge_max_active_duration: u64) {
        assert_one_yocto();
        self.ownership.assert_owner();
        require!(
            badge_max_active_duration > 0,
            "Badge max active duration must be greater than 0"
        );

        self.badge_max_active_duration = badge_max_active_duration;
    }

    pub fn get_badge_min_creation_deposit(&self) -> Balance {
        self.badge_min_creation_deposit
    }

    #[payable]
    pub fn set_badge_min_creation_deposit(&mut self, badge_min_creation_deposit: Balance) {
        assert_one_yocto();
        self.ownership.assert_owner();

        self.badge_min_creation_deposit = badge_min_creation_deposit;
    }

    fn validate_create_proposal(
        &self,
        proposal: &Proposal<BadgeAction>,
        create_request: &BadgeCreate,
    ) {
        // Ensure unique ID
        require!(
            self.badges.get(&create_request.id).is_none(),
            "Badge ID already exists"
        );

        let now = env::block_timestamp();

        // Validate start_at
        require!(
            create_request.start_at.unwrap_or(now) + create_request.duration > now,
            "Badge active period has already ended",
        );

        // Validate duration
        require!(
            create_request.duration <= self.badge_max_active_duration,
            "Exceeded maximum active duration",
        );

        // Validate deposit
        require!(
            proposal.deposit >= self.badge_min_creation_deposit,
            "Deposit does not meet minimum creation deposit requirement",
        );
        require!(
            proposal.deposit
                >= u128::from(billable_days_in_duration(create_request.duration))
                    * self.badge_rate_per_day,
            "Insufficient deposit for specified duration",
        );
    }

    fn validate_extend_proposal(
        &self,
        proposal: &Proposal<BadgeAction>,
        extend_request: &BadgeExtend,
    ) -> Badge {
        let existing_badge = self
            .badges
            .get(&extend_request.id)
            .unwrap_or_else(|| env::panic_str("Badge ID does not exist"));

        require!(
            existing_badge.duration.is_some(),
            "Cannot extend: Existing badge has no duration (indefinite)"
        );

        let now = env::block_timestamp();

        // Validate duration
        require!(
            u64::saturating_sub(
                existing_badge.start_at
                    + existing_badge.duration.unwrap()
                    + extend_request.duration,
                now
            ) <= self.badge_max_active_duration,
            "Exceeded maximum active duration",
        );

        // Validate deposit
        require!(
            proposal.deposit
                >= u128::from(billable_days_in_duration(extend_request.duration))
                    * self.badge_rate_per_day,
            "Insufficient deposit for specified duration",
        );

        existing_badge
    }

    fn on_proposal_change(&mut self, proposal: &Proposal<BadgeAction>) {
        match (&proposal.status, proposal.tag.as_str()) {
            (ProposalStatus::PENDING, TAG_BADGE_CREATE) => {
                let create_request = extract_msg!(proposal, BadgeAction, Create);
                self.validate_create_proposal(proposal, create_request);
            }
            (ProposalStatus::PENDING, TAG_BADGE_EXTEND) => {
                let extend_request = extract_msg!(proposal, BadgeAction, Extend);
                self.validate_extend_proposal(proposal, extend_request);
            }
            (ProposalStatus::ACCEPTED, TAG_BADGE_CREATE) => {
                let create_request = extract_msg!(proposal, BadgeAction, Create);

                self.validate_create_proposal(proposal, create_request);

                let now = env::block_timestamp();

                self.badges.insert(
                    &create_request.id.clone(),
                    &Badge {
                        id: create_request.id.clone(),
                        group_id: create_request.group_id.clone(),
                        name: create_request.name.clone(),
                        description: create_request.description.clone(),
                        created_at: now,
                        start_at: create_request.start_at.unwrap_or(now),
                        duration: Some(create_request.duration),
                        is_enabled: true,
                    },
                );
            }
            (ProposalStatus::ACCEPTED, TAG_BADGE_EXTEND) => {
                let extend_request = extract_msg!(proposal, BadgeAction, Extend);
                let existing_badge = self.validate_extend_proposal(proposal, extend_request);

                self.badges.insert(
                    &existing_badge.id.clone(),
                    &Badge {
                        duration: Some(existing_badge.duration.unwrap() + extend_request.duration),
                        ..existing_badge
                    },
                );
            }
            _ => {}
        }
    }
}

impl_ownership!(StatsGallery, ownership);
impl_sponsorship!(
    StatsGallery,
    sponsorship,
    BadgeAction,
    ownership,
    on_proposal_change
);
