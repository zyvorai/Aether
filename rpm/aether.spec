Name:           aether
Version:        0.1.0
Release:        1%{?dist}
Summary:        Universal runtime control plane

License:        MIT OR Apache-2.0
URL:            https://github.com/zyvorai/Aether
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
Requires:       podman
Recommends:     kubectl

%description
Aether is a universal runtime control plane that deploys workloads
across multiple runtimes: Podman, Kubernetes, and KubeVirt.

Features:
- Single workload specification (YAML)
- Automatic runtime selection
- Zero-downtime migrations between runtimes
- Interactive TUI dashboard
- Prometheus metrics export
- ConfigMaps, Secrets, Ingress, and HPA support

%prep
%autosetup

%build
cargo build --release

%install
# Install binary
install -D -m 0755 target/release/aether %{buildroot}%{_bindir}/aether

# Install schema
install -D -m 0644 schema/workload.schema.json \
    %{buildroot}%{_datadir}/aether/schema/workload.schema.json

# Generate and install shell completions
mkdir -p %{buildroot}%{_datadir}/bash-completion/completions
mkdir -p %{buildroot}%{_datadir}/zsh/site-functions
mkdir -p %{buildroot}%{_datadir}/fish/vendor_completions.d

%{buildroot}%{_bindir}/aether completions bash > \
    %{buildroot}%{_datadir}/bash-completion/completions/aether

%{buildroot}%{_bindir}/aether completions zsh > \
    %{buildroot}%{_datadir}/zsh/site-functions/_aether

%{buildroot}%{_bindir}/aether completions fish > \
    %{buildroot}%{_datadir}/fish/vendor_completions.d/aether.fish

# Install documentation
install -D -m 0644 README.md %{buildroot}%{_docdir}/aether/README.md
install -D -m 0644 CHANGELOG.md %{buildroot}%{_docdir}/aether/CHANGELOG.md
install -D -m 0644 docs/METRICS.md %{buildroot}%{_docdir}/aether/METRICS.md
install -D -m 0644 docs/SCHEMA.md %{buildroot}%{_docdir}/aether/SCHEMA.md

# Install examples
install -D -m 0644 examples/workload-full-featured.yaml \
    %{buildroot}%{_docdir}/aether/examples/workload-full-featured.yaml

%check
cargo test --all

%files
%license LICENSE-MIT LICENSE-APACHE
%doc %{_docdir}/aether/

%{_bindir}/aether
%{_datadir}/aether/schema/workload.schema.json
%{_datadir}/bash-completion/completions/aether
%{_datadir}/zsh/site-functions/_aether
%{_datadir}/fish/vendor_completions.d/aether.fish

%changelog
* Wed Feb 06 2026 ZyvorAI Labs Private Limited <ssahani@gmail.com> - 0.1.0-1
- Initial release
- Core functionality with four runtimes (Podman, Kubernetes, KubeVirt, Metal3)
- Migration engine with three strategies
- Interactive TUI dashboard
- Prometheus metrics support
- Advanced Kubernetes features (ConfigMaps, Secrets, Ingress, HPA)
- Shell completions for bash, zsh, fish, powershell, elvish
