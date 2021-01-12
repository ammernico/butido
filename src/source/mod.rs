use std::path::PathBuf;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use log::trace;
use url::Url;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Source;

#[derive(Clone, Debug)]
pub struct SourceCache {
    root: PathBuf,
}

impl SourceCache {
    pub fn new(root: PathBuf) -> Self {
        SourceCache { root }
    }

    pub fn sources_for(&self, p: &Package) -> Vec<SourceEntry> {
        SourceEntry::for_package(self.root.clone(), p)
    }
}

#[derive(Debug)]
pub struct SourceEntry {
    cache_root: PathBuf,
    package_name: PackageName,
    package_version: PackageVersion,
    package_source_name: String,
    package_source: Source,
}

impl SourceEntry {

    fn source_file_path(&self) -> PathBuf {
        self.source_file_directory().join(format!("{}-{}.source", self.package_source_name, self.package_source.hash().value()))
    }

    fn source_file_directory(&self) -> PathBuf {
        self.cache_root.join(format!("{}-{}", self.package_name, self.package_version))
    }

    fn for_package(cache_root: PathBuf, package: &Package) -> Vec<Self> {
        package.sources()
            .clone()
            .into_iter()
            .map(|(source_name, source)| {
                SourceEntry {
                    cache_root: cache_root.clone(),
                    package_name: package.name().clone(),
                    package_version: package.version().clone(),
                    package_source_name: source_name,
                    package_source: source,
                }
            })
            .collect()
    }

    pub fn exists(&self) -> bool {
        self.source_file_path().exists()
    }

    pub fn path(&self) -> PathBuf {
        self.source_file_path()
    }

    pub fn url(&self) -> &Url {
        self.package_source.url()
    }

    pub async fn remove_file(&self) -> Result<()> {
        let p = self.source_file_path();
        tokio::fs::remove_file(&p).await?;
        Ok(())
    }

    pub async fn verify_hash(&self) -> Result<()> {
        let p = self.source_file_path();
        trace!("Reading: {}", p.display());

        // we can clone() here, because the object itself is just a representation of "what hash
        // type do we use here", which is rather cheap to clone (because it is
        // crate::package::SourceHash, that is not more than an enum + String).
        //
        // We need to clone to move into the closure below.
        let source_hash = self.package_source.hash().clone();

        tokio::task::spawn_blocking(move || {
            std::fs::OpenOptions::new()
                .create(false)
                .create_new(false)
                .read(true)
                .open(&p)
                .map_err(Error::from)
                .map(std::io::BufReader::new)
                .and_then(|reader| {
                    source_hash.matches_hash_of(reader)
                })
        })
        .await?
    }

    pub async fn create(&self) -> Result<tokio::fs::File> {
        let p = self.source_file_path();
        trace!("Creating source file: {}", p.display());

        if !self.cache_root.is_dir() {
            trace!("Cache root does not exist: {}", self.cache_root.display());
            return Err(anyhow!("Cache root {} does not exist!", self.cache_root.display()))
        }

        {
            let dir = self.source_file_directory();
            if !dir.is_dir() {
                trace!("Creating directory: {}", dir.display());
                tokio::fs::create_dir(&dir)
                    .await
                    .with_context(|| {
                        anyhow!("Creating source cache directory for package {} {}: {}",
                            self.package_source_name,
                            self.package_source.hash().value(),
                            dir.display())
                    })?;
            } else {
                trace!("Directory exists: {}", dir.display());
            }
        }

        trace!("Creating file now: {}", p.display());
        tokio::fs::OpenOptions::new()
            .create(true)
            .create_new(true)
            .write(true)
            .open(&p)
            .await
            .with_context(|| anyhow!("Creating file: {}", p.display()))
            .map_err(Error::from)
    }

}

