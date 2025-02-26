//! cli arguments for configuring the evm settings
use clap::{ArgAction, Parser};
use ethers_core::types::{Address, H256, U256};
use eyre::ContextCompat;
use foundry_config::{
    figment::{
        self,
        error::Kind::InvalidType,
        value::{Dict, Map, Value},
        Metadata, Profile, Provider,
    },
    Chain, Config,
};
use serde::Serialize;

/// `EvmArgs` and `EnvArgs` take the highest precedence in the Config/Figment hierarchy.
/// All vars are opt-in, their default values are expected to be set by the
/// [`foundry_config::Config`], and are always present ([`foundry_config::Config::default`])
///
/// Both have corresponding types in the `evm_adapters` crate which have mandatory fields.
/// The expected workflow is
///   1. load the [`foundry_config::Config`]
///   2. merge with `EvmArgs` into a `figment::Figment`
///   3. extract `evm_adapters::Opts` from the merged `Figment`
///
/// # Example
///
/// ```ignore
/// use foundry_config::Config;
/// use forge::executor::opts::EvmOpts;
/// use foundry_common::evm::EvmArgs;
/// # fn t(args: EvmArgs) {
/// let figment = Config::figment_with_root(".").merge(args);
/// let opts = figment.extract::<EvmOpts>().unwrap();
/// # }
/// ```
#[derive(Debug, Clone, Default, Parser, Serialize)]
#[clap(next_help_heading = "EVM options", about = None)] // override doc
pub struct EvmArgs {
    /// Fetch state over a remote endpoint instead of starting from an empty state.
    ///
    /// If you want to fetch state from a specific block number, see --fork-block-number.
    #[clap(long, short, visible_alias = "rpc-url", value_name = "URL")]
    #[serde(rename = "eth_rpc_url", skip_serializing_if = "Option::is_none")]
    pub fork_url: Option<String>,

    /// Fetch state from a specific block number over a remote endpoint.
    ///
    /// See --fork-url.
    #[clap(long, requires = "fork_url", value_name = "BLOCK")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_block_number: Option<u64>,

    /// Initial retry backoff on encountering errors.
    ///
    /// See --fork-url.
    #[clap(long, requires = "fork_url", value_name = "BACKOFF")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_retry_backoff: Option<u64>,

    /// Explicitly disables the use of RPC caching.
    ///
    /// All storage slots are read entirely from the endpoint.
    ///
    /// This flag overrides the project's configuration file.
    ///
    /// See --fork-url.
    #[clap(long)]
    #[serde(skip)]
    pub no_storage_caching: bool,

    /// The initial balance of deployed test contracts.
    #[clap(long, value_name = "BALANCE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_balance: Option<U256>,

    /// The address which will be executing tests.
    #[clap(long, value_name = "ADDRESS")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Address>,

    /// Enable the FFI cheatcode.
    #[clap(help = "Enables the FFI cheatcode.", long)]
    #[serde(skip)]
    pub ffi: bool,

    /// Verbosity of the EVM.
    ///
    /// Pass multiple times to increase the verbosity (e.g. -v, -vv, -vvv).
    ///
    /// Verbosity levels:
    /// - 2: Print logs for all tests
    /// - 3: Print execution traces for failing tests
    /// - 4: Print execution traces for all tests, and setup traces for failing tests
    /// - 5: Print execution and setup traces for all tests
    #[clap(long, short, verbatim_doc_comment, action = ArgAction::Count)]
    #[serde(skip)]
    pub verbosity: u8,

    /// All ethereum environment related arguments
    #[clap(flatten)]
    #[serde(flatten)]
    pub env: EnvArgs,

    /// Sets the number of assumed available compute units per second for this provider
    ///
    /// default value: 330
    ///
    /// See --fork-url.
    /// See also, https://github.com/alchemyplatform/alchemy-docs/blob/master/documentation/compute-units.md#rate-limits-cups
    #[clap(
        long,
        requires = "fork_url",
        alias = "cups",
        value_name = "CUPS",
        help_heading = "Fork config"
    )]
    pub compute_units_per_second: Option<u64>,

    /// Disables rate limiting for this node's provider.
    ///
    /// default value: false
    ///
    /// See --fork-url.
    /// See also, https://github.com/alchemyplatform/alchemy-docs/blob/master/documentation/compute-units.md#rate-limits-cups
    #[clap(
        long,
        requires = "fork_url",
        value_name = "NO_RATE_LIMITS",
        help = "Disables rate limiting for this node provider.",
        help_heading = "Fork config",
        visible_alias = "no-rate-limit"
    )]
    #[serde(skip)]
    pub no_rpc_rate_limit: bool,
}

// Make this set of options a `figment::Provider` so that it can be merged into the `Config`
impl Provider for EvmArgs {
    fn metadata(&self) -> Metadata {
        Metadata::named("Evm Opts Provider")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let value = Value::serialize(self)?;
        let error = InvalidType(value.to_actual(), "map".into());
        let mut dict = value.into_dict().ok_or(error)?;

        if self.verbosity > 0 {
            // need to merge that manually otherwise `from_occurrences` does not work
            dict.insert("verbosity".to_string(), self.verbosity.into());
        }

        if self.ffi {
            dict.insert("ffi".to_string(), self.ffi.into());
        }

        if self.no_storage_caching {
            dict.insert("no_storage_caching".to_string(), self.no_storage_caching.into());
        }

        if self.no_rpc_rate_limit {
            dict.insert("no_rpc_rate_limit".to_string(), self.no_rpc_rate_limit.into());
        }

        if let Some(fork_url) = &self.fork_url {
            dict.insert("eth_rpc_url".to_string(), fork_url.clone().into());
        }

        Ok(Map::from([(Config::selected_profile(), dict)]))
    }
}

/// Configures the executor environment during tests.
#[derive(Debug, Clone, Default, Parser, Serialize)]
#[clap(next_help_heading = "Executor environment config")]
pub struct EnvArgs {
    /// The block gas limit.
    #[clap(long, value_name = "GAS_LIMIT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<u64>,

    /// EIP-170: Contract code size limit in bytes. Useful to increase this because of tests. By
    /// default, it is 0x6000 (~25kb).
    #[clap(long, value_name = "CODE_SIZE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_size_limit: Option<usize>,

    /// The chain ID.
    #[clap(long, alias = "chain", value_name = "CHAIN_ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<Chain>,

    /// The gas price.
    #[clap(long, value_name = "GAS_PRICE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<u64>,

    /// The base fee in a block.
    #[clap(long, visible_alias = "base-fee", value_name = "FEE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_base_fee_per_gas: Option<u64>,

    /// The transaction origin.
    #[clap(long, value_name = "ADDRESS")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_origin: Option<Address>,

    /// The coinbase of the block.
    #[clap(long, value_name = "ADDRESS")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_coinbase: Option<Address>,

    /// The timestamp of the block.
    #[clap(long, value_name = "TIMESTAMP")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_timestamp: Option<u64>,

    /// The block number.
    #[clap(long, value_name = "BLOCK")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,

    /// The block difficulty.
    #[clap(long, value_name = "DIFFICULTY")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_difficulty: Option<u64>,

    /// The block prevrandao value. NOTE: Before merge this field was mix_hash.
    #[clap(long, value_name = "PREVRANDAO")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_prevrandao: Option<H256>,

    /// The block gas limit.
    #[clap(long, value_name = "GAS_LIMIT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_gas_limit: Option<u64>,

    /// The memory limit of the EVM in bytes (32 MB by default)
    #[clap(long, value_name = "MEMORY_LIMIT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<u64>,
}

impl EvmArgs {
    /// Ensures that fork url exists and returns its reference.
    pub fn ensure_fork_url(&self) -> eyre::Result<&String> {
        self.fork_url.as_ref().wrap_err("Missing `--fork-url` field.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_chain_id() {
        let args = EvmArgs {
            env: EnvArgs {
                chain_id: Some(ethers_core::types::Chain::Mainnet.into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::from_provider(Config::figment().merge(args));
        assert_eq!(config.chain_id, Some(ethers_core::types::Chain::Mainnet.into()));

        let env = EnvArgs::parse_from(["foundry-common", "--chain-id", "goerli"]);
        assert_eq!(env.chain_id, Some(ethers_core::types::Chain::Goerli.into()));
    }

    #[test]
    fn test_memory_limit() {
        let args = EvmArgs {
            env: EnvArgs {
                chain_id: Some(ethers_core::types::Chain::Mainnet.into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let config = Config::from_provider(Config::figment().merge(args));
        assert_eq!(config.memory_limit, Config::default().memory_limit);

        let env = EnvArgs::parse_from(["foundry-common", "--memory-limit", "100"]);
        assert_eq!(env.memory_limit, Some(100));
    }
}
