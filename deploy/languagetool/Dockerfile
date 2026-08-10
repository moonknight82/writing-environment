FROM eclipse-temurin:17-jre-jammy@sha256:89e68b9bb83713510b63e2059a415792a7fc77e14b739a7d7ede97f6d9ca2c38

ARG LANGUAGETOOL_VERSION=6.6
ARG LANGUAGETOOL_SHA256=53600506b399bb5ffe1e4c8dec794fd378212f14aaf38ccef9b6f89314d11631

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl unzip \
    && rm -rf /var/lib/apt/lists/* \
    && curl --fail --location --retry 3 \
        "https://languagetool.org/download/LanguageTool-${LANGUAGETOOL_VERSION}.zip" \
        --output /tmp/languagetool.zip \
    && echo "${LANGUAGETOOL_SHA256}  /tmp/languagetool.zip" | sha256sum --check --strict \
    && unzip -q /tmp/languagetool.zip -d /opt \
    && mv "/opt/LanguageTool-${LANGUAGETOOL_VERSION}" /opt/languagetool \
    && rm /tmp/languagetool.zip \
    && groupadd --system languagetool \
    && useradd --system --gid languagetool --home-dir /home/languagetool languagetool \
    && chown -R languagetool:languagetool /opt/languagetool

COPY --chown=languagetool:languagetool server.properties /opt/languagetool/server.properties

USER languagetool
WORKDIR /opt/languagetool

ENV JAVA_TOOL_OPTIONS="-Xms128m -Xmx768m"

EXPOSE 8081

HEALTHCHECK --interval=30s --timeout=5s --start-period=45s --retries=3 \
  CMD curl --fail --silent --show-error http://127.0.0.1:8081/v2/languages >/dev/null || exit 1

CMD ["java", "-cp", "languagetool-server.jar", "org.languagetool.server.HTTPServer", "--config", "server.properties", "--port", "8081", "--public"]
