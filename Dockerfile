# Stage 1: Build the application with MUSL
FROM rust:1.83 AS builder

WORKDIR /app

# Add MUSL target and dependencies
RUN rustup target add x86_64-unknown-linux-musl

# Copy source files and build dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --target=x86_64-unknown-linux-musl || true

RUN rm -rf src && mkdir src
COPY ./src ./src

# Build the actual application
RUN cargo build --release --target=x86_64-unknown-linux-musl

# Stage 2: Minimal runtime image with scratch
FROM scratch

WORKDIR /app

# Copy the statically linked binary from builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/mrml /mrml

# Set executable permissions and entrypoint
ENTRYPOINT ["/mrml"]
