//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'tree-of' subcommand

use std::convert::TryFrom;

use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use resiter::AndThen;

use crate::config::Configuration;
use crate::package::condition::ConditionData;
use crate::package::Dag;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::util::docker::resolve_image_name;
use crate::util::EnvironmentVariableName;

/// Implementation of the "tree_of" subcommand
pub async fn tree_of(matches: &ArgMatches, repo: Repository, config: &Configuration) -> Result<()> {
    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let image_name = matches
        .get_one::<String>("image")
        .map(|s| resolve_image_name(s, config.docker().images()))
        .transpose()?;

    let additional_env = matches
        .get_many::<String>("env")
        .unwrap_or_default()
        .map(AsRef::as_ref)
        .map(crate::util::env::parse_to_env)
        .collect::<Result<Vec<(EnvironmentVariableName, String)>>>()?;

    let condition_data = ConditionData {
        image_name: image_name.as_ref(),
        env: &additional_env,
    };

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .map(|package| Dag::for_root_package(package.clone(), &repo, None, &condition_data))
        .and_then_ok(|tree| {
            let stdout = std::io::stdout();
            let mut outlock = stdout.lock();

            ptree::write_tree(&tree.display(), &mut outlock).map_err(Error::from)
        })
        .collect::<Result<()>>()
}
