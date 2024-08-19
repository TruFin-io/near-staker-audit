#!/bin/bash

# This script provides a set of functions to interact with the NEAR staker contract.
# These functions allow to whitelist users, stake NEAR tokens, allocate and deallocate funds, and view the staker contract state.
# You can use these functions to script sequences of transactions to test the staker contract.
# 
# How to run:
#  STAKER="stakerxyz.trufin.testnet" \
#  OWNER_ID="trufin.testnet" \
#  DEFAULT_DELEGATION_POOL="aurora.pool.f863973.m0" \
#  SECOND_DELEGATION_POOL="pool01b.carlo01.testnet" \
#  USERS_TO_WHITELIST="user1.testnet,user2.testnet,user3.testnet" \
# ./send_transactions.sh


# Set default values if not provided by the environment
export STAKER=${STAKER:-"staker000.trufin.testnet"}
export OWNER_ID=${OWNER_ID:-"trufin.testnet"}
export DEFAULT_DELEGATION_POOL=${DEFAULT_DELEGATION_POOL:-"aurora.pool.f863973.m0"}
export SECOND_DELEGATION_POOL=${SECOND_DELEGATION_POOL:-"pool01b.carlo01.testnet"}

# If USERS_TO_WHITELIST is not set, use default
if [ -z "${USERS_TO_WHITELIST+x}" ]; then
  USERS_TO_WHITELIST=("carlo01.testnet" "carlo02.testnet" "carlo03.testnet" "carlo04.testnet")
else
  IFS=',' read -r -a USERS_TO_WHITELIST <<< "$USERS_TO_WHITELIST"
fi


print_error() {
  local RED='\033[0;31m'
  local NC='\033[0m'
  echo -e "${RED}$1${NC}" >&2
}

print_success() {
  local GREEN='\033[0;32m'
  local NC='\033[0m'
  echo -e "${GREEN}$1${NC}"
}

###### STAKER ACTIONS ######

whitelist() {
  local user=$1

  near call $STAKER add_user_to_whitelist "{\"user_id\": \"$user\"}" --accountId $OWNER_ID --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "Failed to whitelist $user"
  fi

  print_success "$user successfully whitelisted"
}

stake() {
  local user=$1
  local amount=$2

  near call $STAKER stake --accountId $user --amount $amount --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "User $user failed to stake $amount NEAR on default pool $DEFAULT_DELEGATION_POOL"
    exit $?
  fi
  
  print_success "User $user staked $amount NEAR on default pool $DEFAULT_DELEGATION_POOL"
  return 0
}

stake_to_specific_pool() {
  local user=$1
  local amount=$2
  local delegation_pool=$3

  near call $STAKER stake_to_specific_pool "{\"pool_id\": \"$delegation_pool\"}" --accountId $user --amount $amount --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "User $user failed to stake $amount NEAR on pool $delegation_pool"
    exit $?
  fi
  
  print_success "User $user staked $amount NEAR on pool $delegation_pool"
  return 0
}

unstake() {
  local user=$1
  local amount=$2

  local zeros="000000000000000000000000"
  near call $STAKER unstake "{\"amount\": \"$amount$zeros\"}" --accountId $user --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "User $user failed to unstake $amount NEAR"
    exit $?
  fi
  
  print_success "User $user unstaked $amount NEAR"
  return 0
}

unstake_from_specific_pool() {
  local user=$1
  local amount=$2
  local delegation_pool=$3

  local zeros="000000000000000000000000"
  near call $STAKER unstake_from_specific_pool "{\"pool_id\": \"$delegation_pool\", \"amount\": \"$amount$zeros\"}" --accountId $user --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "User $user failed to unstake $amount NEAR from pool $delegation_pool"
    exit $?
  fi
  
  print_success "User $user unstaked $amount NEAR from pool $delegation_pool"
  return 0
}

allocate() {
  local user=$1
  local recipient=$2
  local amount=$3

  local zeros="000000000000000000000000"
  near call $STAKER allocate "{\"recipient\": \"$recipient\", \"amount\": \"$amount$zeros\"}" --accountId $user --gas 300000000000000

  if [ $? -ne 0 ]; then
    print_error "User $user failed to allocate $amount NEAR to $recipient."
    exit $?
  fi
  
  print_success "User $user allocated $amount NEAR to $recipient."
  return 0
}

deallocate() {
  local user=$1
  local recipient=$2
  local amount=$3

  local zeros="000000000000000000000000"
  near call $STAKER deallocate "{\"recipient\": \"$recipient\", \"amount\": \"$amount$zeros\"}" --accountId $user --gas 300000000000000

  if [ $? -ne 0 ]; then
    print_error "User $user failed to deallocate $amount NEAR from $recipient."
    exit $?
  fi
  
  print_success "User $user deallocated $amount NEAR from $recipient."
  return 0
}

update_total_staked() {
  local user=$1

  near call $STAKER update_total_staked --accountId $user --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "User $user failed to call update_total_staked"
    exit $?
  fi
  
  print_success "User $user successfully called update_total_staked"
  return 0
}

withdraw() {
  local user=$1
  local unstake_nonce=$2

  near call $STAKER withdraw "{\"unstake_nonce\": \"$unstake_nonce\"}" --accountId $user --gas 300000000000000
}

distribute_rewards() {
  local user=$1
  local recipient=$2
  local in_near
  local attached_deposit
  if [ "$3" == "IN_NEAR" ]; then
      in_near="true"
      attached_deposit="--amount $4"
  else
      in_near="false"
  fi

  near call $STAKER distribute_rewards "{\"recipient\": \"$recipient\", \"in_near\": $in_near}" $attached_deposit --accountId $user --gas 300000000000000

  if [ $? -ne 0 ]; then
    print_error "User $user failed to distribute_rewards to $recipient in_near $in_near."
    exit $?
  fi
  
  print_success "User $user distribute_rewards to $recipient in_near $in_near."
  return 0
}

distribute_all() {
  local user=$1
  local in_near
  local attached_deposit
  if [ "$2" == "IN_NEAR" ]; then
      in_near="true"
      attached_deposit="--amount $3"
  else
      in_near="false"
  fi

  near call $STAKER distribute_all "{\"in_near\": $in_near}" $attached_deposit --accountId $user --gas 300000000000000

  if [ $? -ne 0 ]; then
    print_error "User $user failed to distribute_all $2."
    exit $?
  fi
  
  print_success "User $user called distribute_all $2 successfully."
  return 0
}


###### STAKER VIEW FUNCTIONS ######

get_staker_info() {
    near view $STAKER get_staker_info --networkId testnet
}

get_total_staked() {
    near view $STAKER get_total_staked --networkId testnet
}

max_withdraw() {
    local user=$1
    near view $STAKER max_withdraw "{\"account_id\": \"$user\"}" --networkId testnet
}

get_share_price() {
    near view $STAKER share_price --networkId testnet
}

get_total_allocated() {
    local user=$1
    near view $STAKER get_total_allocated "{\"allocator\": \"$user\"}" --networkId testnet
}


###### SEND TRANSACTIONS ######

### Whitelist users ###
# for user in "${USERS_TO_WHITELIST[@]}"; do
#   whitelist $user
# done

### Update total staked and share price ###
update_total_staked "carlo01.testnet"

### Stake some NEAR to both pools ###
stake "carlo01.testnet" 50
stake "carlo02.testnet" 50
stake "carlo03.testnet" 50
stake "carlo04.testnet" 50

stake_to_specific_pool "carlo01.testnet" 10 $SECOND_DELEGATION_POOL
stake_to_specific_pool "carlo02.testnet" 5 $SECOND_DELEGATION_POOL 
stake_to_specific_pool "carlo03.testnet" 2 $SECOND_DELEGATION_POOL 
stake_to_specific_pool "carlo04.testnet" 4 $SECOND_DELEGATION_POOL 

### Unstake some NEAR to both pools ###
unstake "carlo01.testnet" 5
unstake_from_specific_pool "carlo02.testnet" 5 $SECOND_DELEGATION_POOL

### Withdraw an unstake nonce ###
withdraw "carlo01.testnet" 1

### Allocate NEAR to users ###
allocate "carlo01.testnet" "carlo02.testnet" 5
allocate "carlo02.testnet" "carlo03.testnet" 5
allocate "carlo02.testnet" "carlo04.testnet" 5

### Deallocate NEAR from users ###
deallocate "carlo01.testnet" "carlo02.testnet" 2
deallocate "carlo02.testnet" "carlo01.testnet" 1


### Distribute rewards ###
distribute_rewards "carlo01.testnet" "carlo02.testnet"
distribute_rewards "carlo02.testnet" "carlo01.testnet" IN_NEAR 1

distribute_all "carlo03.testnet"
distribute_all "carlo04.testnet" IN_NEAR 1

###### ACCESS STAKER STATE ######

get_staker_info
get_share_price
get_total_staked
get_total_allocated "carlo01.testnet"

### Get max withdraw for each user ###
for user in "${users_to_whitelist[@]}"; do
  max_withdraw $user
done
