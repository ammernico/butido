//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use tracing::trace;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

pub fn build_package_filter_by_name(name: PackageName) -> impl filters::filter::Filter<Package> {
    move |p: &Package| {
        trace!("Checking {:?} -> name == {}", p, name);
        *p.name() == name
    }
}

pub fn build_package_filter_by_version_constraint(
    version_constraint: Option<PackageVersionConstraint>,
) -> impl filters::filter::Filter<Package> {
    move |p: &Package| {
        trace!(
            "Checking {:?} -> version matches constraint: {:?}",
            p,
            version_constraint
        );
        version_constraint
            .as_ref()
            .map(|v| v.matches(p.version()))
            .unwrap_or(true)
    }
}
