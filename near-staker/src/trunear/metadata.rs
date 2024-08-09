use crate::*;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};

#[near]
impl FungibleTokenMetadataProvider for NearStaker {
    /// Returns the TruNEAR metadata.
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "TruNEAR Token".to_string(),
            symbol: "TruNEAR".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 24,
        }
    }
}
