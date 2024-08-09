use crate::NearStaker;
use near_sdk::near;

#[near(serializers=[borsh])]
pub enum VersionedNearStaker {
    V1(NearStaker),
}

/// Converts from an old version of the contract to the new one.
impl From<VersionedNearStaker> for NearStaker {
    fn from(contract: VersionedNearStaker) -> Self {
        match contract {
            VersionedNearStaker::V1(state) => state,
        }
    }
}
