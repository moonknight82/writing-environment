# syntax=docker/dockerfile:1.7

FROM debian:trixie AS build

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends gcc libgtk-3-dev pkg-config \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY bluetooth.c ./

RUN mkdir -p /out \
  && cc \
    -std=c11 \
    -O2 \
    -fPIC \
    -shared \
    -Wall \
    -Wextra \
    -Werror \
    $(pkg-config --cflags gtk+-3.0) \
    bluetooth.c \
    -o /out/librpcc_writing-environment-bluetooth.so \
    $(pkg-config --libs gtk+-3.0)

FROM scratch AS export
COPY --from=build /out/ /
