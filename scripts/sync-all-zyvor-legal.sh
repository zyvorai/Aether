#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

# Full Zyvor legal sync: LICENSE, docs/legal, source metadata, tooling scripts.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"
./scripts/sync-proprietary-license.sh
./scripts/sync-legal-framework.sh
./scripts/sync-source-license-metadata.sh
./scripts/sync-zyvor-tooling.sh
python3 ./scripts/sync-tt-zyvor-legal.py
echo ""
echo "All Zyvor legal sync steps complete."
echo "OSS licenses preserved: guestkit + tt/cloud-netconfig, hyper2kvm, hypersdk,"
echo "  hypersdk-org-profile, netctl, netevd."
