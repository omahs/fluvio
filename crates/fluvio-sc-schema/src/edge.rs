#[cfg(feature = "json")]
use std::ops::Deref;

use anyhow::Result;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use fluvio_controlplane::upstream_cluster::UpstreamClusterSpec;
use fluvio_controlplane_metadata::topic::TopicSpec;
use fluvio_stream_model::k8_types::{K8Obj, Spec, ObjectMeta};

#[derive(Debug, Default)]
#[cfg_attr(
    feature = "use_serde",
    derive(Deserialize, Serialize),
    serde(rename_all = "camelCase")
)]
pub struct EdgeMetadata {
    #[cfg_attr(feature = "use_serde", serde(default))]
    pub topics: Vec<K8Obj<TopicSpec>>,
    #[cfg_attr(feature = "use_serde", serde(default))]
    pub upstream_clusters: Vec<K8Obj<UpstreamClusterSpec>>,
}

/// Configuration used to inihilize a Cluster locally. This data is copied to
/// the K8 cluster metadata
#[derive(Debug, Default)]
#[cfg_attr(
    feature = "use_serde",
    derive(Deserialize, Serialize),
    serde(rename_all = "camelCase")
)]
pub struct EdgeMetadataExport {
    #[cfg_attr(feature = "use_serde", serde(default))]
    pub topics: Vec<K8ObjExport<TopicSpec>>,
    #[cfg_attr(feature = "use_serde", serde(default))]
    pub upstream_clusters: Vec<K8ObjExport<UpstreamClusterSpec>>,
}

impl EdgeMetadataExport {
    pub fn new(
        topics: Vec<K8Obj<TopicSpec>>,
        upstream_clusters: Vec<K8Obj<UpstreamClusterSpec>>,
    ) -> Self {
        Self {
            topics: topics.into_iter().map(|t| t.into()).collect(),
            upstream_clusters: upstream_clusters.into_iter().map(|u| u.into()).collect(),
        }
    }
}

impl EdgeMetadata {
    pub fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Represents a ClusterConfig that is read from a file. Usually a JSON file.
#[cfg(feature = "json")]
#[derive(Debug, Default)]
pub struct EdgeMetadataFile(EdgeMetadata);

#[cfg(feature = "json")]
impl EdgeMetadataFile {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)?;

        Self::from_json(&contents)
    }

    fn from_json(json: &str) -> Result<Self> {
        let config: EdgeMetadata = serde_json::from_str(json)?;

        config.validate()?;

        Ok(Self(config))
    }
}

#[cfg(feature = "json")]
impl Deref for EdgeMetadataFile {
    type Target = EdgeMetadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "json")]
impl From<EdgeMetadataFile> for EdgeMetadata {
    fn from(file: EdgeMetadataFile) -> Self {
        file.0
    }
}

#[derive(Debug)]
#[cfg_attr(
    feature = "use_serde",
    derive(Deserialize, Serialize),
    serde(rename_all = "camelCase"),
    serde(bound(serialize = "S: Serialize")),
    serde(bound(deserialize = "S: DeserializeOwned"))
)]
pub struct K8ObjExport<S>
where
    S: Spec,
{
    #[cfg_attr(feature = "use_serde", serde(default = "S::api_version"))]
    pub api_version: String,
    #[cfg_attr(feature = "use_serde", serde(default = "S::kind"))]
    pub kind: String,
    #[cfg_attr(feature = "use_serde", serde(default))]
    pub metadata: ObjectMeta,
    #[cfg_attr(feature = "use_serde", serde(default))]
    pub spec: S,
}

impl<S: Spec> From<K8Obj<S>> for K8ObjExport<S> {
    fn from(obj: K8Obj<S>) -> Self {
        Self {
            api_version: obj.api_version,
            kind: obj.kind,
            metadata: obj.metadata,
            spec: obj.spec,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EdgeMetadata, EdgeMetadataFile};

    #[test]
    fn validates_json_config() {
        let config = r#"{
            "topics": [
              {
                "apiVersion": "fluvio.infinyon.com/v2",
                "kind": "Topic",
                "metadata": {
                  "name": "my-topic"
                },
                "spec": {
                  "replicas": {
                    "computed": {
                      "partitions": 1,
                      "replicationFactor": 1,
                      "ignoreRackAssignment": false
                    }
                  }
                }
              }
            ]
          }
          "#;

        let config = EdgeMetadataFile::from_json(config);

        assert!(config.is_ok());

        let config: EdgeMetadata = config.unwrap().into();

        assert!(config.topics.len() == 1);
    }
}
