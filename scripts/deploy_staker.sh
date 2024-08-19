#!/bin/bash

# What this script does:
# 1. builds the staker contract
# 2. deploys and initialises the contract under the $STAKER account 
# 3. adds a second delegation pool 
# 4. whitelists some users
# 
# How to run:
#  STAKER="stakerxyz.trufin.testnet" \
#  OWNER_ID="trufin.testnet" \
#  TREASURY="treasury.trufin.testnet" \
#  DEFAULT_DELEGATION_POOL="aurora.pool.f863973.m0" \
#  SECOND_DELEGATION_POOL="pool01b.carlo01.testnet" \
#  USERS_TO_WHITELIST="user1.testnet,user2.testnet,user3.testnet" \
# ./deploy_staker.sh


# Set default values if not provided by the environment
export STAKER=${STAKER:-"staker000.trufin.testnet"}
export OWNER_ID=${OWNER_ID:-"trufin.testnet"}
export TREASURY=${TREASURY:-"treasury.trufin.testnet"}
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

add_pool() {
  local delegation_pool=$1

  near call $STAKER add_pool "{\"pool_id\": \"$delegation_pool\"}" --accountId $OWNER_ID --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "Failed to add pool $delegation_pool"
    exit $?
  fi
  
  print_success "Added pool $delegation_pool"
  return 0
}

whitelist() {
  local user=$1
  near call $STAKER add_user_to_whitelist "{\"user_id\": \"$user\"}" --accountId $OWNER_ID --gas 300000000000000
  if [ $? -ne 0 ]; then
    print_error "Failed to whitelist $user"
    exit $?
  fi

  print_success "$user successfully whitelisted"
}

### Build the contract ###
make clean && make build
if [ $? -ne 0 ]; then
  exit $?
fi

### Create account ###
near create-account $STAKER --masterAccount $OWNER_ID  --initialBalance 10
if [ $? -ne 0 ]; then
  print_error "Failed to create NEAR account $STAKER"
  exit $?
fi

### Deploy contract ###
echo "Deploying contract to account $STAKER. owner_id: $OWNER_ID, treasury: $TREASURY, default_delegation_pool: $DEFAULT_DELEGATION_POOL"

near deploy $STAKER ./res/near_staker.wasm --initFunction new --initArgs "{\"owner_id\": \"$OWNER_ID\", \"treasury\": \"$TREASURY\", \"default_delegation_pool\": \"$DEFAULT_DELEGATION_POOL\"}"
if [ $? -ne 0 ]; then
  print_error "Failed to deploy contract under account $STAKER"
  exit $?
fi
print_success "Contract deployed successfully!"

### Add second pool ###
echo "Adding second delegation pool..."
add_pool $SECOND_DELEGATION_POOL

### Whitelist users ###
for user in "${USERS_TO_WHITELIST[@]}"; do
  whitelist $user
done
