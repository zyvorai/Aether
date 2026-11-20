FROM debian:bookworm-slim

# kubectl is used by cluster watch/exec/port-forward subprocess sessions
# in src/api/handlers.rs. helm is used by the Helm App Store / release
# management handlers in src/kubecluster.rs and src/helm.rs.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libgcc-s1 curl tar && \
    KUBECTL_VERSION="$(curl -fsSL https://dl.k8s.io/release/stable.txt)" && \
    curl -fsSL -o /usr/local/bin/kubectl "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/$(dpkg --print-architecture)/kubectl" && \
    chmod +x /usr/local/bin/kubectl && \
    ARCH="$(dpkg --print-architecture)" && \
    HELM_VERSION="$(curl -fsSL https://api.github.com/repos/helm/helm/releases/latest | grep -oP '"tag_name":\s*"\K[^"]+')" && \
    curl -fsSL "https://get.helm.sh/helm-${HELM_VERSION}-linux-${ARCH}.tar.gz" | tar -xz -C /tmp && \
    mv "/tmp/linux-${ARCH}/helm" /usr/local/bin/helm && \
    chmod +x /usr/local/bin/helm && \
    rm -rf "/tmp/linux-${ARCH}" && \
    apt-get purge -y curl && apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 65532 -g root nonroot

COPY aether /usr/local/bin/aether

USER 65532

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/aether"]
CMD ["serve"]
