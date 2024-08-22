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

use std::collections::HashMap;

use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use petgraph::dot::Dot;
use petgraph::graph::DiGraph;
use petgraph::visit::IntoNodeIdentifiers;
use resiter::AndThen;

use crate::config::Configuration;
use crate::package::condition::ConditionData;
use crate::package::Dag;
use crate::package::DependencyType;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::util::docker::ImageNameLookup;
use crate::util::EnvironmentVariableName;

fn convert_dag_to_petgraph(dag: Dag) -> DiGraph<String, DependencyType> {
    let mut graph = DiGraph::new();
    let mut node_map = HashMap::new();

    for node_idx in dag.dag().node_identifiers() {
        if let Some(node_weight) = dag.dag().node_weight(node_idx) {
            let pet_node = graph.add_node(node_weight.clone().display_name_version());
            node_map.insert(node_idx, pet_node);
        }
    }

    for edge in dag.dag().raw_edges() {
        let source_node = node_map[&edge.source()];
        let target_node = node_map[&edge.target()];
        graph.add_edge(source_node, target_node, edge.weight.clone());
    }

    graph
}

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

    let image_name_lookup = ImageNameLookup::create(config.docker().images())?;
    let image_name = matches
        .get_one::<String>("image")
        .map(|s| image_name_lookup.expand(s))
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

    let dot = matches.get_flag("dot");

    let package_dags = repo
        .packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .map(|package| Dag::for_root_package(package.clone(), &repo, None, &condition_data));

    if dot {
        for dag in package_dags {
            let petgraph = convert_dag_to_petgraph(dag.unwrap());

            fn get_edge_color(weight: &DependencyType) -> &str {
                match weight {
                    DependencyType::Build => "orange",
                    DependencyType::Runtime => "blue",
                }
            }

            let dot = Dot::with_attr_getters(
                &petgraph,
                &[petgraph::dot::Config::EdgeNoLabel, petgraph::dot::Config::NodeNoLabel],
                &|_, nr| format!("color = {} ", get_edge_color(nr.weight())).to_string(),
                &|_, node| format!("label = {} ", node.1),
            );
            println!("{:?}", dot);
        }
        return Ok(());
    }

    package_dags
        .and_then_ok(|tree| {
            let stdout = std::io::stdout();
            let mut outlock = stdout.lock();

            ptree::write_tree(&tree.display(), &mut outlock).map_err(Error::from)
        })
        .collect::<Result<()>>()
}
