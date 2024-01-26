use std::sync::Arc;

use clap::ValueEnum;
use clap::Parser;
use common::installation::InstallationType;
use fluvio::config::ConfigFile;
use semver::Version;
use tracing::debug;

mod check;
mod delete;
mod diagnostics;
mod error;
mod group;
mod shutdown;
mod spu;
mod start;
mod status;
mod remote_cluster;
mod util;

use check::CheckOpt;
use delete::DeleteOpt;
use diagnostics::DiagnosticsOpt;
use group::SpuGroupCmd;
use remote_cluster::RemoteClusterOpt;
use shutdown::ShutdownOpt;
use spu::SpuCmd;
use start::StartOpt;
use status::StatusOpt;

pub use self::error::ClusterCliError;

use anyhow::Result;

use fluvio_extension_common as common;
use common::target::ClusterTarget;
use common::output::Terminal;
use fluvio_channel::{ImageTagStrategy, FLUVIO_IMAGE_TAG_STRATEGY};

pub(crate) const VERSION: &str = include_str!("../../../../VERSION");

/// Manage and view Fluvio clusters
#[derive(Debug, Parser)]
pub enum ClusterCmd {
    /// Install Fluvio cluster
    #[command(name = "start")]
    Start(Box<StartOpt>),

    /// Uninstall a Fluvio cluster
    #[command(name = "delete")]
    Delete(DeleteOpt),

    /// Check that all requirements for cluster startup are met.
    ///
    /// This command is useful to check if user has all the required dependencies and permissions to run
    /// fluvio cluster.
    #[command(name = "check")]
    Check(CheckOpt),

    /// Manage and view Streaming Processing Units (SPUs)
    ///
    /// SPUs make up the part of a Fluvio cluster which is in charge
    /// of receiving messages from producers, storing those messages,
    /// and relaying them to consumers. This command lets you see
    /// the status of SPUs in your cluster.
    #[command(subcommand, name = "spu")]
    SPU(SpuCmd),

    /// Manage and view SPU Groups (SPGs)
    ///
    /// SPGs are groups of SPUs in a cluster which are managed together.
    #[command(subcommand, name = "spg")]
    SPUGroup(SpuGroupCmd),

    /// Collect anonymous diagnostic information to help with debugging
    #[command(name = "diagnostics")]
    Diagnostics(DiagnosticsOpt),

    /// Check the status of a Fluvio cluster
    #[command(name = "status")]
    Status(StatusOpt),

    /// Shutdown cluster processes without deleting data
    #[command(name = "shutdown")]
    Shutdown(ShutdownOpt),

    /// Remote-cluster commands
    #[command(
        subcommand,
        name = "remote-cluster",
        visible_alias = "rem",
        alias = "remote"
    )]
    RemoteCluster(RemoteClusterOpt),
}

impl ClusterCmd {
    /// process cluster commands
    pub async fn process<O: Terminal>(
        self,
        out: Arc<O>,
        platform_version: Version,
        target: ClusterTarget,
    ) -> Result<()> {
        match self {
            Self::Start(mut start) => {
                if let Ok(tag_strategy_value) = std::env::var(FLUVIO_IMAGE_TAG_STRATEGY) {
                    let tag_strategy = ImageTagStrategy::from_str(&tag_strategy_value, true)
                        .unwrap_or(ImageTagStrategy::Version);
                    match tag_strategy {
                        ImageTagStrategy::Version => {
                            debug!("Using image version: {}", VERSION);
                        }
                        ImageTagStrategy::VersionGit => {
                            let image_version = format!("{}-{}", VERSION, env!("GIT_HASH"));
                            debug!("Using image version: {:?}", &image_version);
                            start.k8_config.image_version = Some(image_version);
                        }
                        ImageTagStrategy::Git => {
                            debug!("Using developer image version: {}", env!("GIT_HASH"));
                            start.develop = true
                        }
                    }
                };

                start.process(platform_version, false).await?;
            }
            Self::Delete(uninstall) => {
                uninstall.process().await?;
            }
            Self::Check(check) => {
                check.process(platform_version).await?;
            }
            Self::SPU(spu) => {
                let fluvio = target.connect().await?;
                spu.process(out, &fluvio).await?;
            }
            Self::SPUGroup(group) => {
                let fluvio = target.connect().await?;
                group.process(out, &fluvio).await?;
            }
            Self::Diagnostics(opt) => {
                opt.process().await?;
            }
            Self::Status(status) => {
                status.process(target).await?;
            }
            Self::Shutdown(opt) => {
                opt.process().await?;
            }
            Self::RemoteCluster(opt) => {
                opt.execute(out, target).await?;
            }
        }

        Ok(())
    }
}

pub(crate) fn get_installation_type() -> Result<InstallationType, ClusterCliError> {
    let config = ConfigFile::load_default_or_new()?;
    Ok(InstallationType::load_or_default(
        config.config().current_cluster()?,
    ))
}
