// Copyright 2019-2023 Parity Technologies (UK) Ltd.
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

use clap::Parser as ClapParser;
use codec::Decode;
use color_eyre::eyre::{self, WrapErr};
use frame_metadata::{
    v15::RuntimeMetadataV15, RuntimeMetadata, RuntimeMetadataPrefixed, META_RESERVED,
};
use jsonrpsee::client_transport::ws::Uri;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use subxt_codegen::utils::MetadataVersion;
use subxt_metadata::{get_metadata_hash, get_pallet_hash, metadata_v14_to_latest};

/// Verify metadata compatibility between substrate nodes.
#[derive(Debug, ClapParser)]
pub struct Opts {
    /// Urls of the substrate nodes to verify for metadata compatibility.
    #[clap(name = "nodes", long, use_value_delimiter = true, value_parser)]
    nodes: Vec<Uri>,
    /// Check the compatibility of metadata for a particular pallet.
    ///
    /// ### Note
    /// The validation will omit the full metadata check and focus instead on the pallet.
    #[clap(long, value_parser)]
    pallet: Option<String>,
    /// Specify the metadata version.
    ///
    ///  - unstable:
    ///
    ///    Use the latest unstable metadata of the node.
    ///
    ///  - number
    ///
    ///    Use this specific metadata version.
    ///
    /// Defaults to latest.
    #[clap(long = "version", default_value = "latest")]
    version: MetadataVersion,
}

pub async fn run(opts: Opts) -> color_eyre::Result<()> {
    match opts.pallet {
        Some(pallet) => {
            handle_pallet_metadata(opts.nodes.as_slice(), pallet.as_str(), opts.version).await
        }
        None => handle_full_metadata(opts.nodes.as_slice(), opts.version).await,
    }
}

async fn handle_pallet_metadata(
    nodes: &[Uri],
    name: &str,
    version: MetadataVersion,
) -> color_eyre::Result<()> {
    #[derive(Serialize, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    struct CompatibilityPallet {
        pallet_present: HashMap<String, Vec<String>>,
        pallet_not_found: Vec<String>,
    }

    let mut compatibility: CompatibilityPallet = Default::default();
    for node in nodes.iter() {
        let metadata = fetch_runtime_metadata(node, version).await?;

        match metadata.pallets.iter().find(|pallet| pallet.name == name) {
            Some(pallet_metadata) => {
                let hash = get_pallet_hash(&metadata.types, pallet_metadata);
                let hex_hash = hex::encode(hash);
                println!("Node {node:?} has pallet metadata hash {hex_hash:?}");

                compatibility
                    .pallet_present
                    .entry(hex_hash)
                    .or_insert_with(Vec::new)
                    .push(node.to_string());
            }
            None => {
                compatibility.pallet_not_found.push(node.to_string());
            }
        }
    }

    println!(
        "\nCompatible nodes by pallet\n{}",
        serde_json::to_string_pretty(&compatibility)
            .context("Failed to parse compatibility map")?
    );

    Ok(())
}

async fn handle_full_metadata(nodes: &[Uri], version: MetadataVersion) -> color_eyre::Result<()> {
    let mut compatibility_map: HashMap<String, Vec<String>> = HashMap::new();
    for node in nodes.iter() {
        let metadata = fetch_runtime_metadata(node, version).await?;
        let hash = get_metadata_hash(&metadata);
        let hex_hash = hex::encode(hash);
        println!("Node {node:?} has metadata hash {hex_hash:?}",);

        compatibility_map
            .entry(hex_hash)
            .or_insert_with(Vec::new)
            .push(node.to_string());
    }

    println!(
        "\nCompatible nodes\n{}",
        serde_json::to_string_pretty(&compatibility_map)
            .context("Failed to parse compatibility map")?
    );

    Ok(())
}

async fn fetch_runtime_metadata(
    url: &Uri,
    version: MetadataVersion,
) -> color_eyre::Result<RuntimeMetadataV15> {
    let bytes = subxt_codegen::utils::fetch_metadata_bytes(url, version).await?;

    let metadata = <RuntimeMetadataPrefixed as Decode>::decode(&mut &bytes[..])?;
    if metadata.0 != META_RESERVED {
        return Err(eyre::eyre!(
            "Node {:?} has invalid metadata prefix: {:?} expected prefix: {:?}",
            url,
            metadata.0,
            META_RESERVED
        ));
    }

    match metadata.1 {
        RuntimeMetadata::V14(v14) => Ok(metadata_v14_to_latest(v14)),
        RuntimeMetadata::V15(v15) => Ok(v15),
        _ => Err(eyre::eyre!(
            "Node {:?} with unsupported metadata version: {:?}",
            url,
            metadata.1
        )),
    }
}
