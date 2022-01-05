use crate::impl_ownership;
use crate::*;

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    OWNERSHIP,
    SPONSORSHIP,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct StatsGallery {
    ownership: Ownership,
    sponsorship: Sponsorship,
}

#[near_bindgen]
impl StatsGallery {
    #[init]
    pub fn new(owner_id: AccountId, sponsorship_tags: Vec<String>, proposal_duration: u64) -> Self {
        Self {
            ownership: Ownership::new(StorageKey::OWNERSHIP, owner_id),
            sponsorship: Sponsorship::new(
                StorageKey::SPONSORSHIP,
                sponsorship_tags,
                Some(proposal_duration),
            ),
        }
    }

    fn test_cb(&self, proposal: &Proposal) {
        log!("Test callback on proposal: {}", proposal.id);
    }
}

impl_ownership!(StatsGallery, ownership);
impl_sponsorship!(StatsGallery, sponsorship, ownership, test_cb);
