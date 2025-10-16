# Multi-stage build for easytier-bridge workspace
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/easytier-bridge

# Copy workspace configuration
COPY Cargo.toml ./
COPY build_all.sh ./

# Copy all crates
COPY easytier_common ./easytier_common
COPY easytier_device_client ./easytier_device_client
COPY easytier_network_gateway ./easytier_network_gateway
COPY easytier_config_server ./easytier_config_server

# Build all crates
RUN chmod +x build_all.sh && \
    cargo build --all --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false easytier

# Copy the libraries and headers
COPY --from=builder /usr/src/easytier-bridge/target/release/libeasytier_common.so /usr/local/lib/
COPY --from=builder /usr/src/easytier-bridge/target/release/libeasytier_device_client.so /usr/local/lib/
COPY --from=builder /usr/src/easytier-bridge/easytier_common/include/easytier_common.h /usr/local/include/
COPY --from=builder /usr/src/easytier-bridge/easytier_device_client/include/easytier_device_client.h /usr/local/include/

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
    CMD test -f /usr/local/lib/libeasytier_device_client.so || exit 1

# Default command
CMD ["/bin/bash", "-c", "echo 'EasyTier Bridge container is ready. Libraries available at /usr/local/lib/libeasytier_*.so'"]