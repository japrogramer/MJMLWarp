# Stage 1: Build the application
FROM rust:1.83 AS builder

# Set the working directory
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files to set up dependencies
COPY Cargo.toml Cargo.lock ./

# Cache dependencies to avoid redundant downloads
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release  || true

# Remove the dummy `main.rs`
RUN rm -rf src && mkdir src

# Copy the real source code
COPY ./src ./src

# Force a clean build to ensure the real source is built
RUN rm -rf target
RUN cargo build --release

# Stage 2: Run the application using a minimal image with updated GLIBC
FROM ubuntu:22.04

# Install required libraries for GLIBC compatibility
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/mrml /app/mrml

# Ensure binary is executable
RUN chmod +x /app/mrml

# Copy optional templates directory
COPY ./templates /app/templates

# Expose the port the app will run on
EXPOSE 3030

# Run the application
CMD ["./mrml"]
