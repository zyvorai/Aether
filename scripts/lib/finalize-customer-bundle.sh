#!/usr/bin/env bash
# Finalize customer tarball: branded PDFs, welcome page, path verification.
# Usage: finalize-customer-bundle.sh <stage> <build-dir> <product> [version]
set -euo pipefail

STAGE="${1:?stage directory}"
BUILD_DIR="${2:?build directory}"
PRODUCT="${3:?product name}"
VERSION="${4:-${V9S_PACKAGE_VERSION:-latest}}"
LIB="${BUILD_DIR}/scripts/lib"

# License pack (LICENSE, LEGAL-INDEX.txt, docs/legal/, Zyvor terms when applicable)
if [[ -x "${LIB}/copy-legal-to-bundle.sh" ]]; then
  "${LIB}/copy-legal-to-bundle.sh" "${STAGE}" "${BUILD_DIR}"
elif [[ -x "${LIB}/copy-zyvor-legal-to-bundle.sh" ]]; then
  extra=()
  [[ -f "${BUILD_DIR}/ZYVOR-COMPANY-TERMS.md" ]] && extra=(--with-accept)
  "${LIB}/copy-zyvor-legal-to-bundle.sh" "${STAGE}" "${BUILD_DIR}" "${extra[@]}"
fi
if [[ -f "${LIB}/license-accept.sh" ]]; then
  mkdir -p "${STAGE}/.package-lib"
  cp "${LIB}/license-accept.sh" "${STAGE}/.package-lib/"
  chmod +x "${STAGE}/.package-lib/license-accept.sh"
fi

for tool in generate-customer-pdfs.sh verify-bundle-script-paths.sh; do
  [[ -x "${LIB}/${tool}" ]] || { echo "ERROR: missing ${LIB}/${tool}" >&2; exit 1; }
done

chmod +x "${LIB}/generate-customer-pdfs.sh" "${LIB}/verify-bundle-script-paths.sh"
"${LIB}/generate-customer-pdfs.sh" "${STAGE}" "${BUILD_DIR}" "${PRODUCT}" "${VERSION}"
"${LIB}/verify-bundle-script-paths.sh" "${STAGE}"

test -f "${STAGE}/docs/welcome.html" || { echo "ERROR: missing docs/welcome.html" >&2; exit 1; }
test -f "${STAGE}/docs/pdf/WELCOME.pdf" || { echo "ERROR: missing docs/pdf/WELCOME.pdf" >&2; exit 1; }
test -f "${STAGE}/OPEN_FIRST.txt" || { echo "ERROR: missing OPEN_FIRST.txt" >&2; exit 1; }
