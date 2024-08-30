use std::collections::BTreeMap;

use borsh::{BorshDeserialize, BorshSerialize};
use casper_macros::CasperABI;
use casper_sdk::{collections::Map, host::Entity, types::Address};
use serde::{Deserialize, Serialize};

// Metadata mutability is different from schema mutability.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub(crate) struct MetadataSchemaProperty {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub(crate) struct CustomMetadataSchema {
    pub properties: BTreeMap<String, MetadataSchemaProperty>,
}

// Using a structure for the purposes of serialization formatting.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub(crate) struct MetadataNFT721 {
    pub name: String,
    pub symbol: String,
    pub token_uri: String,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub(crate) struct MetadataCEP78 {
    pub name: String,
    pub token_uri: String,
    pub checksum: String,
}

// Using a structure for the purposes of serialization formatting.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub(crate) struct CustomMetadata {
    pub attributes: BTreeMap<String, String>,
}

// VM2 doesn't support nested containers, so Map<E, Vec<E>> is
// not really possible - this is a workaround around this issue.
// Just store Vec<{E, E}> instead of Map<E, Vec<E>>. This makes
// some data redundant, but it's the best we can do for now.
#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub struct OperatorEntry {
    pub key: Entity,
    pub value: Entity,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Default, Debug, Clone)]
pub struct TokenData {
    pub approved: Option<Entity>,
    pub issuer: Option<Entity>,
    pub owner: Option<Entity>,
    pub metadata: String,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Default, Debug, Clone)]
pub struct EntityData {
    pub balance: u64,
    pub whitelisted: bool,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone)]
pub struct StateStore {
    pub operators: Vec<OperatorEntry>,
    pub entity_data: Map<Entity, EntityData>,
    pub data: Map<TokenIdentifier, TokenData>,
    pub hash_by_index: Map<u64, String>,
    pub index_by_hash: Map<String, u64>,
    pub burned_tokens: Vec<TokenIdentifier>,
    pub json_schema: Option<String>,
    pub metadata: Map<TokenIdentifier, String>,
}

impl Default for StateStore {
    fn default() -> Self {
        let operators = Vec::new();
        let entity_data = Map::new("ENTITY_DATA");
        let data = Map::new("TOKEN_DATA");
        let metadata = Map::new("STORE_METADATA");
        let hash_by_index = Map::new("STORE_HASH_BY_INDEX");
        let index_by_hash = Map::new("STORE_INDEX_BY_HASH");
        let burned_tokens = Vec::new();
        let json_schema = None;

        Self {
            operators,
            entity_data,
            data,
            metadata,
            hash_by_index,
            index_by_hash,
            burned_tokens,
            json_schema,
        }
    }
}

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
    pub burn_mode: BurnMode,
    pub operator_burn_mode: bool,

    pub store: StateStore,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum NFTIdentifierMode {
    Ordinal = 0,
    Hash = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum EventsMode {
    NoEvents = 0,
    CEP47 = 1,
    CES = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum MetadataMutability {
    Immutable = 0,
    Mutable = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum BurnMode {
    Burnable = 0,
    NonBurnable = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum OwnerReverseLookupMode {
    NoLookUp = 0,
    Complete = 1,
    TransfersOnly = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum NFTMetadataKind {
    CEP78 = 0,
    NFT721 = 1,
    Raw = 2,
    CustomValidated = 3,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum MintingMode {
    Installer = 0,
    /// The ability to mint NFTs is not restricted.
    Public = 1,
    /// The ability to mint NFTs is restricted by an ACL.
    Acl = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum OwnershipMode {
    /// The minter owns it and can never transfer it.
    Minter = 0,
    /// The minter assigns it to an address and can never be transferred.
    Assigned = 1,
    /// The NFT can be transferred even to an recipient that does not exist.
    Transferable = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
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
#[borsh(use_discriminant = true)]
pub enum NFTHolderMode {
    Accounts = 0,
    Contracts = 1,
    Mixed = 2,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
#[borsh(use_discriminant = true)]
pub enum WhitelistMode {
    Unlocked = 0,
    Locked = 1,
}

#[derive(BorshSerialize, BorshDeserialize, CasperABI, Debug, Clone, PartialEq)]
pub enum TokenIdentifier {
    Ordinal(u64),
    Hash(String),
}

impl TokenIdentifier {
    pub fn to_string(&self) -> String {
        match self {
            TokenIdentifier::Ordinal(ord) => ord.to_string(),
            TokenIdentifier::Hash(hash) => hash.clone(),
        }
    }
}
