# Stage 1: Build the application
FROM rust:1.71 AS builder

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

# Stage 2: Run the application using a minimal image
FROM debian:bullseye-slim

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
