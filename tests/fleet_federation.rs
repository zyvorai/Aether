// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

use aether::fleet::federation::federation_policies;
use std::env;

#[test]
fn federation_policies_parse_env() {
    env::set_var("AETHER_FEDERATION_CLUSTERS", "prod,staging");
    env::set_var("AETHER_FEDERATION_WEIGHTS", "prod=3,staging=1");
    let p = federation_policies();
    assert_eq!(p.clusters, vec!["prod", "staging"]);
    assert_eq!(p.weights.get("prod"), Some(&3.0));
    env::remove_var("AETHER_FEDERATION_CLUSTERS");
    env::remove_var("AETHER_FEDERATION_WEIGHTS");
}
