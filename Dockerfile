FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libgcc-s1 && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 65532 -g root nonroot

COPY aether /usr/local/bin/aether

USER 65532

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/aether"]
CMD ["serve"]
