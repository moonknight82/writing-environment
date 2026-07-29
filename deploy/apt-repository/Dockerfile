FROM debian:trixie

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    apt-utils \
    ca-certificates \
    dpkg-dev \
    file \
    gnupg \
    gpgv \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
