FROM node:20-bookworm AS build

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH=/root/.cargo/bin:$PATH

ARG TARGETARCH
RUN test "$TARGETARCH" = "arm64"

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    file \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    wget \
  && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal
RUN corepack enable && corepack prepare pnpm@10.17.1 --activate

WORKDIR /workspace
COPY . .

RUN pnpm install --frozen-lockfile
ARG BUILD_ID
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
  && pnpm tauri build --bundles deb $updater_args \
  && scripts/pi/package-artifact.sh /artifacts "$BUILD_ID"

FROM scratch AS export
COPY --from=build /artifacts/ /
