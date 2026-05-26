# Demo 03: KubeVirt → Metal3

Migrate a high-resource workload from KubeVirt VM to bare metal (Metal3).

## Prerequisites

- KubeVirt cluster
- Metal3 / BareMetalHost inventory (lab environment)

## Steps

```bash
aether validate --spec workload-kubevirt.yaml
aether run --spec workload-kubevirt.yaml --runtime kubevirt
aether migrate demo-kubevirt-metal3 metal3 --strategy rolling
aether status demo-kubevirt-metal3
```

## Expected output

- Rolling strategy steps logged with `--verbose-trace`
- Final runtime `metal3`

## Rollback

```bash
aether migrate demo-kubevirt-metal3 kubevirt --strategy blue-green
```

## Honest limits

Metal3 lab setups vary; this demo documents the **spec + migrate path**, not automated BMC provisioning.
