use borsh::{BorshDeserialize, BorshSerialize};
use casper_macros::CasperABI;
use casper_sdk::{host::Entity, types::Address};

use crate::error::NFTCoreError;

const MAX_TOTAL_TOKEN_SUPPLY: u64 = 100_000_000;

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub struct CEP78State {
    pub collection_name: String,
    pub collection_symbol: String,
    pub total_token_supply: u64,
    pub allow_minting: bool,
    pub minting_mode: MintingMode,
    pub ownership_mode: OwnershipMode,
    pub nft_kind: NFTKind,
    pub whitelist_mode: WhitelistMode,
    pub acl_whitelist: Vec<Address>,
    pub acl_package_mode: bool,
    pub package_operator_mode: bool,
    pub package_hash: String,
    pub base_metadata_kind: NFTMetadataKind,
    pub optional_metadata: Vec<u8>,
    pub additional_required_metadata: Vec<u8>,
    pub identifier_mode: NFTIdentifierMode,
    pub metadata_mutability: MetadataMutability,

    pub installer: Entity,
    pub events_mode: EventsMode,
    pub minted_tokens_count: u64,
    pub owned_tokens_count: u64,
    pub burn_mode: BurnMode,
    pub operator_burn_mode: bool
    // pub reporting_mode: OwnerReverseLookupMode,
    // pub transfer_filter_contract_contract_hash: Option<Address>,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum NFTIdentifierMode {
    Ordinal = 0,
    Hash = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum EventsMode {
    NoEvents = 0,
    CEP47 = 1,
    CES = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum MetadataMutability {
    Immutable = 0,
    Mutable = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum BurnMode {
    Burnable = 0,
    NonBurnable = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum OwnerReverseLookupMode {
    NoLookUp = 0,
    Complete = 1,
    TransfersOnly = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum NFTMetadataKind {
    CEP78 = 0,
    NFT721 = 1,
    Raw = 2,
    CustomValidated = 3,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum MintingMode {
    Installer = 0,
    /// The ability to mint NFTs is not restricted.
    Public = 1,
    /// The ability to mint NFTs is restricted by an ACL.
    Acl = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum OwnershipMode {
    /// The minter owns it and can never transfer it.
    Minter = 0,
    /// The minter assigns it to an address and can never be transferred.
    Assigned = 1,
    /// The NFT can be transferred even to an recipient that does not exist.
    Transferable = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum NFTKind {
    /// The NFT represents a real-world physical
    /// like a house.
    Physical = 0,
    /// The NFT represents a digital asset like a unique
    /// JPEG or digital art.
    Digital = 1,
    /// The NFT is the virtual representation
    /// of a physical notion, e.g a patent
    /// or copyright.
    Virtual = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum NFTHolderMode {
    Accounts = 0,
    Contracts = 1,
    Mixed = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant=true)]
pub enum WhitelistMode {
    Unlocked = 0,
    Locked = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
pub enum TokenIdentifier {
    Ordinal(u64),
    Hash(String)
}

impl TokenIdentifier {
    pub fn to_string(&self) -> String {
        match self {
            TokenIdentifier::Ordinal(ord) => ord.to_string(),
            TokenIdentifier::Hash(hash) => hash.clone(),
        }
    }
}