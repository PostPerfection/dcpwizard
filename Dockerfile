# DCP Wizard — Headless batch processing container
# Usage:
#   docker build -t dcpwizard .
#   docker run -v /path/to/media:/data dcpwizard create --title "My Film" --video /data/j2k --output /data/out
#
# For REST API mode:
#   docker run -p 8080:8080 -v /path/to/media:/data dcpwizard serve --port 8080

FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    ninja-build \
    pkg-config \
    libopenjp2-7-dev \
    libssl-dev \
    libxml2-dev \
    libxerces-c-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN cmake -B build -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_TESTING=OFF \
    -DBUILD_PYTHON_BINDINGS=OFF \
    && cmake --build build --parallel

# --- Runtime image ---
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    libopenjp2-7 \
    libssl3 \
    libxml2 \
    libxerces-c3.2 \
    ffmpeg \
    openjpeg-tools \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/build/dcpwizard /usr/local/bin/dcpwizard

# Create non-root user for security
RUN useradd -m -s /bin/bash dcpwizard
USER dcpwizard
WORKDIR /data

EXPOSE 8080

ENTRYPOINT ["dcpwizard"]
CMD ["--help"]
