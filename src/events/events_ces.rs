use casper_sdk::{casper::Entity, types::Address};

use crate::types::TokenIdentifier;

pub trait Event {}

impl Event for Mint {}
impl Event for Burn {}
impl Event for Approval {}
impl Event for ApprovalRevoked {}
impl Event for ApprovalForAll {}
impl Event for RevokedForAll {}
impl Event for Transfer {}
impl Event for MetadataUpdated {}
impl Event for VariablesSet {}
impl Event for Migration {}

#[derive(Debug, PartialEq, Eq)]
pub struct Mint {
    recipient: Address,
    token_id: String,
    data: String,
}

impl Mint {
    pub fn new(recipient: Address, token_id: TokenIdentifier, data: String) -> Self {
        Self {
            recipient,
            token_id: token_id.to_string(),
            data,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Burn {
    owner: Entity,
    token_id: String,
    burner: Entity,
}

impl Burn {
    pub fn new(owner: Entity, token_id: TokenIdentifier, burner: Entity) -> Self {
        Self {
            owner,
            token_id: token_id.to_string(),
            burner,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Approval {
    owner: Address,
    spender: Address,
    token_id: String,
}

impl Approval {
    pub fn new(owner: Address, spender: Address, token_id: TokenIdentifier) -> Self {
        Self {
            owner,
            spender,
            token_id: token_id.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ApprovalRevoked {
    owner: Address,
    token_id: String,
}

impl ApprovalRevoked {
    pub fn new(owner: Address, token_id: TokenIdentifier) -> Self {
        Self {
            owner,
            token_id: token_id.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ApprovalForAll {
    owner: Address,
    operator: Address,
}

impl ApprovalForAll {
    pub fn new(owner: Address, operator: Address) -> Self {
        Self { owner, operator }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RevokedForAll {
    owner: Address,
    operator: Address,
}

impl RevokedForAll {
    pub fn new(owner: Address, operator: Address) -> Self {
        Self { owner, operator }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Transfer {
    owner: Address,
    spender: Option<Address>,
    recipient: Address,
    token_id: String,
}

impl Transfer {
    pub fn new(
        owner: Address,
        spender: Option<Address>,
        recipient: Address,
        token_id: TokenIdentifier,
    ) -> Self {
        Self {
            owner,
            spender,
            recipient,
            token_id: token_id.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MetadataUpdated {
    token_id: String,
    data: String,
}

impl MetadataUpdated {
    pub fn new(token_id: TokenIdentifier, data: String) -> Self {
        Self {
            token_id: token_id.to_string(),
            data,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct VariablesSet {}

impl VariablesSet {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Migration {}

impl Migration {
    pub fn new() -> Self {
        Self {}
    }
}
