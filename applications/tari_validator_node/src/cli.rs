//  Copyright 2022. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::net::SocketAddr;

use clap::Parser;
use minotari_app_utilities::common_cli_args::CommonCliArgs;
use tari_common::configuration::{ConfigOverrideProvider, Network};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(flatten)]
    pub common: CommonCliArgs,
    /// Enable tracing
    #[clap(long, aliases = &["tracing", "enable-tracing"])]
    pub tracing_enabled: bool,
    /// Bind address for JSON-rpc server
    #[clap(long, alias = "rpc-address")]
    pub json_rpc_address: Option<SocketAddr>,
    #[clap(long, env = "TARI_VN_UI_CONNECT_ADDRESS")]
    pub ui_connect_address: Option<String>,
    /// A replacement of a template address with a local WASM file, in the format <template_address>=<local file path>.
    /// FOR DEBUGGING PURPOSES ONLY
    #[clap(long, short = 'd')]
    pub debug_templates: Vec<String>,
}

impl ConfigOverrideProvider for Cli {
    fn get_config_property_overrides(&self, default_network: Network) -> Vec<(String, String)> {
        let mut overrides = self.common.get_config_property_overrides(default_network);
        let network = self.common.network.unwrap_or(default_network);
        overrides.push(("network".to_string(), network.to_string()));
        overrides.push(("validator_node.override_from".to_string(), network.to_string()));
        overrides.push(("p2p.seeds.override_from".to_string(), network.to_string()));

        if let Some(ref json_rpc_address) = self.json_rpc_address {
            overrides.push((
                "validator_node.json_rpc_address".to_string(),
                json_rpc_address.to_string(),
            ));
        }
        if let Some(ref ui_connect_address) = self.ui_connect_address {
            overrides.push((
                "validator_node.ui_connect_address".to_string(),
                ui_connect_address.to_string(),
            ));
        }
        overrides
    }
}
