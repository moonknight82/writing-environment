FROM debian:trixie

ARG RPI_IMAGE_GEN_COMMIT
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git xz-utils \
  && rm -rf /var/lib/apt/lists/*

RUN git clone https://github.com/raspberrypi/rpi-image-gen.git /opt/rpi-image-gen \
  && git -C /opt/rpi-image-gen checkout "$RPI_IMAGE_GEN_COMMIT" \
  && apt-get update \
  && /opt/rpi-image-gen/install_deps.sh \
  && rm -rf /var/lib/apt/lists/*

COPY image-entrypoint.sh /usr/local/bin/writing-environment-image-entrypoint
COPY finalize-root-device.sh /usr/local/sbin/writing-environment-finalize-root-device
RUN chmod 0755 \
  /usr/local/bin/writing-environment-image-entrypoint \
  /usr/local/sbin/writing-environment-finalize-root-device

WORKDIR /opt/rpi-image-gen
ENTRYPOINT ["/usr/local/bin/writing-environment-image-entrypoint"]
