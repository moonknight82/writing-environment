# Debian 12 is the oldest supported build baseline for the portable amd64
# artifacts. Building against it avoids unnecessarily requiring a newer glibc.
FROM node:20-bookworm AS build

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH=/root/.cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    file \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    patchelf \
    wget \
  && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal
RUN corepack enable && corepack prepare pnpm@10.17.1 --activate

WORKDIR /workspace
COPY . .

RUN pnpm install --frozen-lockfile
ARG TAURI_BUNDLES=deb
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/workspace/src-tauri/target \
    --mount=type=secret,id=tauri_signing_private_key,required=false \
  if test -s /run/secrets/tauri_signing_private_key; then \
    export TAURI_SIGNING_PRIVATE_KEY="$(cat /run/secrets/tauri_signing_private_key)"; \
    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""; \
    updater_args=""; \
  else \
    updater_args='--config {"bundle":{"createUpdaterArtifacts":false}}'; \
  fi \
  && rm -rf src-tauri/target/release/bundle \
  && pnpm tauri build --verbose --bundles "$TAURI_BUNDLES" $updater_args \
  && mkdir -p /artifacts \
  && find src-tauri/target/release/bundle -type f \
    \( -name '*.AppImage' -o -name '*.deb' -o -name '*.sig' \) \
    -exec cp '{}' /artifacts/ \; \
  && test "$(find /artifacts -type f | wc -l)" -ge 1 \
  && cd /artifacts \
  && sha256sum * >SHA256SUMS

FROM scratch AS export
COPY --from=build /artifacts/ /
