# Use the official Rust image to build the project
FROM rust:latest AS builder

# Set the working directory
WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Update rust and cargo to the latest stable version
RUN rustup update stable

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./
# Copy the source code
COPY src ./src

# Build the project
RUN cargo build --release

# Use a smaller base image to run the compiled binary
FROM debian:buster-slim

# Install required libraries
RUN apt-get update && apt-get install -y \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/rai-endpoint-simulator ./
# Copy the zresponse folder
COPY --from=builder /app/zresponse ./zresponse

# Expose the port the application runs on
EXPOSE 4545

# Run the binary
CMD ["./rai-endpoint-simulator"]