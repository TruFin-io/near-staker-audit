#!/bin/bash

# A script to upgrade a staker contract on the testnet
# 
# Usage: ./scripts/upgrade_testnet.sh STAKER OWNER_ID
# Example: ./scripts/upgrade_testnet.sh stakerxyz.trufin.testnet trufin.testnet

STAKER=$1
OWNER_ID=$2

# print usage if the expected arguments are not provided
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 STAKER OWNER_ID"
    echo "       $0 stakerxyz.trufin.testnet trufin.testnet"
    exit 1
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

# build the contract
make build
if [ $? -ne 0 ]; then
  print_error "Failed to build the contract"
  exit $?
fi

# encode the contract binary to base64
base64 res/near_staker.wasm > near_staker_base64.txt
if [ $? -ne 0 ]; then
  print_error "Failed to produce the base64 encoded binary"
  exit $?
fi

# upgrade the contract and perform migration
NEW_CONTRACT_CODE=$(cat near_staker_base64.txt)
near call $STAKER upgrade "{ \"code\": \"$NEW_CONTRACT_CODE\", \"migrate\": true }" --accountId $OWNER_ID --gas 300000000000000
if [ $? -ne 0 ]; then
  print_error "Failed to upgrade the contract"
  exit $?
fi

print_success "Contract $STAKER successfully upgraded"
