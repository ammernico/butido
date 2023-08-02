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

use anyhow::Result;
use clap::ArgMatches;
use tracing::{error, debug};

use crate::package::condition::ConditionCheckable;
use crate::package::condition::ConditionData;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::ParseDependency;
use crate::repository::Repository;
use crate::util::docker::ImageName;
use crate::util::EnvironmentVariableName;

#[derive(Debug, PartialEq)]
enum DependencyType {
    Buildtime,
    Runtime,
}

#[derive(Debug)]
struct DependenciesNode {
    name: String,
    dependencies: Vec<(DependenciesNode, DependencyType)>,
}

fn print_dependencies_tree(node: DependenciesNode, level: usize, is_runtime_dep: bool) {
    debug!("{:?};{:?};{:?}", node, level, is_runtime_dep);
    let ident = "  ".repeat(level);
    let name = node.name;
    let suffix = if is_runtime_dep { "*" } else { "" };
    println!("{ident}- {name}{suffix}");
    for (node, dep_type) in node.dependencies {
        print_dependencies_tree(node, level + 1, dep_type == DependencyType::Runtime);
    }
}

fn build_dependencies_tree(
    p: Package,
    repo: &Repository,
    conditional_data: &ConditionData<'_>,
) -> Result<DependenciesNode, anyhow::Error> {
    /// helper fn with bad name to check the dependency condition of a dependency and parse the dependency into a tuple of
    /// name and version for further processing
    fn process<D: ConditionCheckable + ParseDependency>(
        d: &D,
        conditional_data: &ConditionData<'_>,
        dependency_type: DependencyType,
    ) -> Result<(bool, PackageName, PackageVersionConstraint, DependencyType)> {
        // Check whether the condition of the dependency matches our data
        let take = d.check_condition(conditional_data)?;
        let (name, version) = d.parse_as_name_and_version()?;

        // (dependency check result, name of the dependency, version of the dependency)
        Ok((take, name, version, dependency_type))
    }

    /// Helper fn to get the dependencies of a package
    ///
    /// This function helps getting the dependencies of a package as an iterator over
    /// (Name, Version).
    ///
    /// It also filters out dependencies that do not match the `conditional_data` passed and
    /// makes the dependencies unique over (name, version).
    fn get_package_dependencies<'a>(
        package: &'a Package,
        conditional_data: &'a ConditionData<'_>,
    ) -> impl Iterator<Item = anyhow::Result<(PackageName, PackageVersionConstraint, DependencyType)>> + 'a
    {
        package
            .dependencies()
            .build()
            .iter()
            .map(move |d| process(d, conditional_data, DependencyType::Buildtime))
            .chain({
                package
                    .dependencies()
                    .runtime()
                    .iter()
                    .map(move |d| process(d, conditional_data, DependencyType::Runtime))
            })
            // Now filter out all dependencies where their condition did not match our
            // `conditional_data`.
            .filter(|res| match res {
                Ok((true, _, _, _)) => true,
                Ok((false, _, _, _)) => false,
                Err(_) => true,
            })
            // Map out the boolean from the condition, because we don't need that later on
            .map(|res| res.map(|(_, name, vers, deptype)| (name, vers, deptype)))
    }

    let mut d: Vec<(DependenciesNode, DependencyType)> = Vec::new();
    let deps = get_package_dependencies(&p, conditional_data);
    for dep in deps {
        let dep = match dep {
            Ok(d) => {
                debug!("Found dependency {} {}", d.0, d.1);
                d
            }
            Err(e) => {
                error!("Dependency not ok {}", e);
                continue;
            }
        };

        let package_name = dep.0;
        let package_version_constraint = dep.1;
        let package_dependency_type = dep.2;

        debug!(
            "Searching for ({}, {}) in repo",
            package_name,
            package_version_constraint
        );

        let pkgs = repo.find_with_version(&package_name, &package_version_constraint);
        let pkg = match pkgs.len() {
            0 => {
                debug!(
                    "Package not found in repo: ({}, {})",
                    package_name, package_version_constraint
                );
                continue;
            }
            1 => {
                debug!(
                    "Found one package in repo for: ({}, {})",
                    package_name,
                    package_version_constraint
                );
                pkgs[0]
            }
            _ => {
                debug!(
                    "Found multiple packages in repo for ({}, {}), taking first one",
                    package_name,
                    package_version_constraint
                );
                pkgs[0]
            }
        };

        let subtree = build_dependencies_tree(pkg.clone(), repo, conditional_data);
        debug!("{:?}", subtree);
        let subtree = match subtree {
            Ok(s) => {
                //debug!("Subtree ok, {:?}", subtree);
                debug!("Subtree ok, {:?}", pkg);
                s
            }
            Err(e) => {
                error!("Failed to build subtree, {}", e);
                continue;
            }
        };
        d.push((subtree, package_dependency_type));
    }

    debug!("d.len: {:?}", d.len());
    let tree = DependenciesNode {
        name: p.name().to_string(),
        dependencies: d,
    };
    debug!("tree: {:?}", tree);
    debug!("tree.dependencies: {:?}", tree.dependencies);
    Ok(tree)
}

/// Implementation of the "tree_of" subcommand
pub async fn tree_of(matches: &ArgMatches, repo: Repository) -> Result<()> {
    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);

    let pvers = matches.get_one::<String>("package_version");
    //match pvers {
    //    Some(v) => {
    //        debug!("Called with version: {}", v);
    //    }
    //    _ => {
    //        error!("Please specify a version of package with: packagename =version");
    //        std::process::exit(1);
    //    }
    //};

    let pvers = pvers
        .map(|s| s.to_owned())
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let image_name = matches
        .get_one::<String>("image")
        .map(|s| s.to_owned())
        .map(ImageName::from);

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

    let tree = repo
        .packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .map(|package| {
            let tree = build_dependencies_tree(package.clone(), &repo, &condition_data);
            debug!("{:?}", tree);
            tree
        });

    let mut tree: Vec<DependenciesNode> = tree
        .filter_map(|tree| tree.ok())
        .collect::<Vec<DependenciesNode>>();

    let popped = tree.pop();
    let popped = match popped {
        Some(p) => {
            debug!("Popped tree");
            p
        }
        _ => {
            debug!("Tree empty, nothing found");
            return Ok(());
        }
    };

    print_dependencies_tree(popped, 0, false);
    Ok(())
}
