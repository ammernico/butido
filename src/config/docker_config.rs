//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;

use getset::{CopyGetters, Getters};
use serde::Deserialize;

use crate::config::Endpoint;
use crate::config::EndpointName;
use crate::util::docker::ContainerImage;

/// Configuration of the Docker daemon interfacing functionality
#[derive(Debug, Getters, CopyGetters, Deserialize)]
pub struct DockerConfig {
    /// The required Docker version
    ///
    /// If not set, it will not be checked, which might result in weird things?
    ///
    /// # Note
    ///
    /// Because the Docker API returns strings, not a version object, each compatible version must
    /// be listed.
    #[getset(get = "pub")]
    docker_versions: Option<Vec<String>>,

    /// The required Docker API version
    ///
    /// If not set, it will not be checked, which might result in weird things?
    ///
    /// # Note
    ///
    /// Because the Docker API returns strings, not a version object, each compatible version must
    /// be listed.
    #[getset(get = "pub")]
    docker_api_versions: Option<Vec<String>>,

    #[getset(get = "pub")]
    images: Vec<ContainerImage>,

    #[getset(get = "pub")]
    endpoints: HashMap<EndpointName, Endpoint>,
}
