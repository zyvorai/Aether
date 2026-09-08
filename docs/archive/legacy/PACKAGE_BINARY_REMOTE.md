# Package Aether as a client tarball

## Customer tarball

`install.sh`, `QUICKSTART.txt`, `README.txt`, `aether` (embedded dashboard), `aether.env.example`, test scripts.

## Build

```bash
./scripts/package-binary-remote.sh HOST USER --fetch
```

## Customer install

```bash
tar xzf aether-*-linux-amd64.tar.gz && cd aether-*-linux-amd64
./install.sh
./aether serve --host 0.0.0.0 --port 5090
```

Dashboard: `https://<host>:5090/web/dashboard/`
