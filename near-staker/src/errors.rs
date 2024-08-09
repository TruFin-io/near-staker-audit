/// Common errors ///

pub const ERR_NOT_INITIALIZED: &str = "Contract is not initialized";
pub const ERR_NOT_IN_SYNC: &str = "Contract is not in sync";

/// Staker errors ///

// access errors
pub const ERR_ONLY_OWNER: &str = "Only the owner can call this method";
pub const ERR_PAUSED: &str = "Contract is paused";
pub const ERR_NOT_PAUSED: &str = "Contract is not paused";
pub const ERR_INVALID_CALLER: &str = "Invalid caller";

// staker info errors
pub const ERR_FEE_TOO_LARGE: &str = "Fee cannot be larger than fee precision";
pub const ERR_MIN_DEPOSIT_TOO_SMALL: &str = "Minimum deposit amount is too small";
pub const ERR_STAKE_BELOW_MIN_DEPOSIT: &str = "Deposit amount is below minimum deposit";
pub const ERR_NO_PENDING_OWNER: &str = "No pending owner set";
pub const ERR_NOT_PENDING_OWNER: &str = "Only the pending owner can claim ownership";

// delegation pool errors
pub const ERR_POOL_ALREADY_EXISTS: &str = "Delegation pool already exists";
pub const ERR_POOL_DOES_NOT_EXIST: &str = "Delegation pool does not exist";
pub const ERR_POOL_ALREADY_ENABLED: &str = "Delegation pool already enabled";
pub const ERR_POOL_ALREADY_DISABLED: &str = "Delegation pool already disabled";
pub const ERR_POOL_NOT_ENABLED: &str = "Delegation pool not enabled";
pub const ERR_INSUFFICIENT_FUNDS_ON_POOL: &str = "Insufficient funds on delegation pool";

// allocation errors
pub const ERR_ALLOCATION_UNDER_ONE_NEAR: &str = "Allocated amount must be at least 1 NEAR";
pub const ERR_INVALID_RECIPIENT: &str = "Cannot allocate to this recipient";
pub const ERR_NO_ALLOCATIONS: &str = "User has no allocations";

// user errors
pub const ERR_INVALID_UNSTAKE_AMOUNT: &str = "Invalid unstake amount";
pub const ERR_UNSTAKE_LOCKED: &str = "Unstake is currently locked for this pool";
pub const ERR_NO_ALLOCATIONS_TO_RECIPIENT: &str = "User has no allocations to this recipient";
pub const ERR_INSUFFICIENT_NEAR_BALANCE: &str = "Attached deposit too small";
pub const ERR_EXCESSIVE_DEALLOCATION: &str = "Cannot deallocate more than is allocated";
pub const ERR_INVALID_NONCE: &str = "Invalid nonce";
pub const ERR_INSUFFICIENT_TRUNEAR_BALANCE: &str = "Insufficient TruNEAR balance";
pub const ERR_UNSTAKE_AMOUNT_TOO_LOW: &str = "Unstake amount is too low";
pub const ERR_SENDER_MUST_BE_RECEIVER: &str = "Sender must have requested the unlock";
pub const ERR_WITHDRAW_NOT_READY: &str = "Withdraw not ready";
pub const ERR_INSUFFICIENT_STAKER_BALANCE: &str = "Insufficient staker balance for withdrawal";
pub const ERR_STORAGE_DEPOSIT_TOO_SMALL: &str =
    "The attached deposit is less than the storage cost";

// execution errors
pub const ERR_CALLBACK_FAILED: &str = "Callback failed";
pub const ERR_STAKE_FAILED: &str = "Staking failed";

/// Whitelist errors ///

// agent errors
pub const ERR_CALLER_NOT_AGENT: &str = "Caller is not an agent";
pub const ERR_OWNER_CANNOT_BE_ADDED: &str = "Owner cannot be added as an agent";
pub const ERR_OWNER_CANNOT_BE_REMOVED: &str = "Owner cannot be removed as an agent";
pub const ERR_AGENT_ALREADY_EXISTS: &str = "Agent already exists";
pub const ERR_AGENT_DOES_NOT_EXIST: &str = "Agent does not exist";

// whitelist and blacklist errors
pub const ERR_USER_ALREADY_WHITELISTED: &str = "User already whitelisted";
pub const ERR_USER_ALREADY_BLACKLISTED: &str = "User already blacklisted";
pub const ERR_USER_STATUS_ALREADY_CLEARED: &str = "User status already cleared";
pub const ERR_USER_NOT_WHITELISTED: &str = "User not whitelisted";
