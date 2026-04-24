# Next Steps

## Current State

Aether now has a native Kubernetes and KubeVirt browser with:

- multi-cluster kubeconfig discovery
- workload and resource browsing
- manifest inspect/edit/apply
- delete/restart/start/stop/suspend/resume actions where supported
- scale controls for scalable workloads
- KubeVirt `VirtualMachine`, `VirtualMachineInstance`, and CDI `DataVolume` support
- sanitized cluster `Secret` inspection

## Remaining Work

The main remaining gaps are the interactive and release-management features:

- pod exec and terminal streaming
- port-forward support
- live watch / streaming updates for resource changes
- Helm release browsing and lifecycle actions from the Aether UI
- richer event correlation from resource detail views

## Recommended Next Order

1. Add pod exec and terminal sessions to the cluster browser.
2. Add port-forward creation and teardown for pods and services.
3. Add Helm release inventory and release actions.
4. Add watch-based live refresh for selected resources and workloads.

## Notes

- Keep `Secret` values redacted in cluster detail responses.
- Keep KubeVirt support inside the same cluster browser instead of splitting into a separate VM UI.
- Prefer expanding the native Aether backend and UX instead of reintroducing Headlamp dependencies.
