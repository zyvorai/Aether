Name:           orchestr8
Version:        0.1.0
Release:        1%{?dist}
Summary:        Universal runtime control plane

License:        MIT OR Apache-2.0
URL:            https://github.com/ssahani/orchestr8
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
Requires:       podman
Recommends:     kubectl

%description
Orchestr8 is a universal runtime control plane that deploys workloads
across multiple runtimes: Podman, Kubernetes, KubeVirt, and Metal3.

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
install -D -m 0755 target/release/orchestr8 %{buildroot}%{_bindir}/orchestr8

# Install schema
install -D -m 0644 schema/workload.schema.json \
    %{buildroot}%{_datadir}/orchestr8/schema/workload.schema.json

# Generate and install shell completions
mkdir -p %{buildroot}%{_datadir}/bash-completion/completions
mkdir -p %{buildroot}%{_datadir}/zsh/site-functions
mkdir -p %{buildroot}%{_datadir}/fish/vendor_completions.d

%{buildroot}%{_bindir}/orchestr8 completions bash > \
    %{buildroot}%{_datadir}/bash-completion/completions/orchestr8

%{buildroot}%{_bindir}/orchestr8 completions zsh > \
    %{buildroot}%{_datadir}/zsh/site-functions/_orchestr8

%{buildroot}%{_bindir}/orchestr8 completions fish > \
    %{buildroot}%{_datadir}/fish/vendor_completions.d/orchestr8.fish

# Install documentation
install -D -m 0644 README.md %{buildroot}%{_docdir}/orchestr8/README.md
install -D -m 0644 CHANGELOG.md %{buildroot}%{_docdir}/orchestr8/CHANGELOG.md
install -D -m 0644 docs/METRICS.md %{buildroot}%{_docdir}/orchestr8/METRICS.md
install -D -m 0644 docs/SCHEMA.md %{buildroot}%{_docdir}/orchestr8/SCHEMA.md

# Install examples
install -D -m 0644 examples/workload-full-featured.yaml \
    %{buildroot}%{_docdir}/orchestr8/examples/workload-full-featured.yaml

%check
cargo test --all

%files
%license LICENSE-MIT LICENSE-APACHE
%doc %{_docdir}/orchestr8/

%{_bindir}/orchestr8
%{_datadir}/orchestr8/schema/workload.schema.json
%{_datadir}/bash-completion/completions/orchestr8
%{_datadir}/zsh/site-functions/_orchestr8
%{_datadir}/fish/vendor_completions.d/orchestr8.fish

%changelog
* Wed Feb 06 2026 Susant Sahani <ssahani@gmail.com> - 0.1.0-1
- Initial release
- Core functionality with four runtimes (Podman, Kubernetes, KubeVirt, Metal3)
- Migration engine with three strategies
- Interactive TUI dashboard
- Prometheus metrics support
- Advanced Kubernetes features (ConfigMaps, Secrets, Ingress, HPA)
- Shell completions for bash, zsh, fish, powershell, elvish
