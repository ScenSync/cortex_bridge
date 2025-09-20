# Multi-stage build for easytier-bridge
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/easytier-bridge

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY cbindgen.toml build.rs ./
COPY build_headers.sh ./

# Copy source code
COPY src ./src
COPY resources ./resources
COPY include ./include

# Build the application
RUN chmod +x build_headers.sh && \
    cargo build --release --all-features

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false easytier

# Copy the binary and resources
COPY --from=builder /usr/src/easytier-bridge/target/release/libeasytier_bridge.so /usr/local/lib/
COPY --from=builder /usr/src/easytier-bridge/include/easytier_bridge.h /usr/local/include/
COPY --from=builder /usr/src/easytier-bridge/resources /opt/easytier-bridge/resources

# Set library path
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Create necessary directories
RUN mkdir -p /var/log/easytier-bridge && \
    chown easytier:easytier /var/log/easytier-bridge

# Switch to app user
USER easytier

# Expose default ports (if applicable)
EXPOSE 11010 11011

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD test -f /usr/local/lib/libeasytier_bridge.so || exit 1

# Default command
CMD ["/bin/bash", "-c", "echo 'EasyTier Bridge container is ready. Library available at /usr/local/lib/libeasytier_bridge.so'"]