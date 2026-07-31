FROM debian:bookworm-slim

# kubectl is used by cluster watch/exec/port-forward subprocess sessions
# in src/api/handlers.rs.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libgcc-s1 curl && \
    KUBECTL_VERSION="$(curl -fsSL https://dl.k8s.io/release/stable.txt)" && \
    curl -fsSL -o /usr/local/bin/kubectl "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/$(dpkg --print-architecture)/kubectl" && \
    chmod +x /usr/local/bin/kubectl && \
    apt-get purge -y curl && apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 65532 -g root nonroot

COPY aether /usr/local/bin/aether

USER 65532

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/aether"]
CMD ["serve"]
