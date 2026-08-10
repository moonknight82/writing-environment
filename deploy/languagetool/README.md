# Self-hosted LanguageTool for Writing Environment

This package builds LanguageTool from the official standalone 6.6 archive. It does not depend on an
unofficial LanguageTool container image. The archive checksum is pinned in the Dockerfile, and the
Java 17 base image supports both 64-bit ARM and x86 Docker hosts.

The resulting server provides LanguageTool's basic local rules. LanguageTool's cloud-only AI rules
are not part of the standalone server.

## NAS requirements

- Docker or Portainer on a 64-bit ARM or x86 NAS;
- TCP port 8081 available on the trusted private network;
- at least 1 GB of memory available to the container during checks;
- outbound internet access while building the image, so Docker can download the pinned official
  LanguageTool archive and Java base image.

The compose configuration gives Java a 768 MB maximum heap. Change `JAVA_TOOL_OPTIONS` in
`compose.yaml` if a larger installation needs a different ceiling.

## Build and run

From this directory:

```sh
docker compose -f compose-build.yaml build
docker compose up -d
docker compose ps
```

In Portainer, upload this directory or use it as a Git-backed stack, build `compose-build.yaml`, then
deploy `compose.yaml`. The runtime stack is read-only, drops Linux capabilities, and keeps only its
temporary files in memory.

For an offline prebuilt image, import the Docker archive under **Images > Import**, tag it
`writing-environment-languagetool:6.6`, then create a stack named
`writing-environment-languagetool` by uploading `compose.yaml`. This is the workflow verified on an
x86-64 Synology NAS with Portainer CE 2.39.5 LTS.

Test it from another machine on the LAN without sending manuscript text:

```sh
curl http://NAS_ADDRESS:8081/v2/languages
```

In Writing Environment, open Review, enter
`http://NAS_ADDRESS:8081/v2/check`, select the manuscript language, and choose **Test connection**.
For ordinary LAN HTTP, the app requires a separate acknowledgement that text is not encrypted in
transit. Put the LanguageTool server behind an HTTPS reverse proxy if the network is not fully
trusted; never expose port 8081 directly to the public internet.

## Updating

LanguageTool stopped publishing fixed-version ZIP releases after 6.6 and now publishes daily
snapshots. Keeping 6.6 pinned makes this deployment reproducible. A future update should change the
download URL and checksum together, rebuild the image, verify all four supported languages, and only
then change the runtime image tag.
