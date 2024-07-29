use borsh::{BorshDeserialize, BorshSerialize};
use casper_macros::{casper, CasperABI};
use casper_sdk::{host::Entity, types::Address};

use crate::{error::NFTCoreError, types::TokenIdentifier};

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum CEP47Event {
    Mint {
        recipient: Address,
        token_id: TokenIdentifier,
    },
    Burn {
        owner: Entity,
        token_id: TokenIdentifier,
        burner: Entity,
    },
    ApprovalGranted {
        owner: Address,
        spender: Address,
        token_id: TokenIdentifier,
    },
    ApprovalRevoked {
        owner: Address,
        token_id: TokenIdentifier,
    },
    ApprovalForAll {
        owner: Address,
        operator: Address,
    },
    RevokedForAll {
        owner: Address,
        operator: Address,
    },
    Transfer {
        sender: Address,
        recipient: Address,
        token_id: TokenIdentifier,
    },
    MetadataUpdate {
        token_id: TokenIdentifier,
    },
    VariablesSet,
    Migrate,
}