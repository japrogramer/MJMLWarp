# MUSL MRML Server 

This project renders MJML templates using Handlebars.

## Build And Run

```bash
docker-compose up
```

## Curl Examples

### Convert MJML using provided MJML

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "mjml": "<mjml><mj-body><mj-text>Hello, world!</mj-text></mj-body></mjml>",
    "payload": {}
  }' \
  http://localhost:3030/convert
```

### Convert MJML using a template

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {},
    "template": "my-template.mjml"
  }' \
  http://localhost:3030/convert
```

### Upload Template

```bash
curl -X POST \
  -F "file=@./example.mjml;filename=example.mjml;type=text/plain" \
  http://localhost:3030/templates
```

### List Templates

```bash
curl http://localhost:3030/templates
```

### Key notes

The Dockerfile creates a tiny and fast Rust container image using static linking with MUSL and a `scratch` base.

**Key Optimizations:**

*   **MUSL Static Linking:** Generates a self-contained executable, eliminating runtime dependencies and significantly reducing image size.
*   **`scratch` Base Image:** Starts with an empty image, resulting in the smallest possible footprint. Only the executable is included in the final image.
*   **Dependency Caching:** Leverages Docker's caching mechanism for faster build times by separating dependency installation from source code changes.

**Benefits:**

*   **Small Image Size:**  Reduces storage space and bandwidth usage.
*   **Faster Deployment:** Smaller images download and start quickly.
*   **Improved Security:** Reduced attack surface due to the minimal operating system.

# Template Cache

The template cache is a crucial component for optimizing the performance of MJML to HTML conversions.

## Overview

A template cache to store frequently accessed MJML templates in memory. This avoids the overhead of repeatedly reading templates from disk and recompiling them. The cache employs an auto-expiration policy to ensure that templates are periodically refreshed, even if there are no file system changes. A file system watcher also triggers cache invalidation on template file changes.

## Key Features

*   **In-Memory Cache:** Templates are stored in a `HashMap` in memory for fast retrieval.
*   **Auto-Expiration:** Templates are automatically removed from the cache if they haven't been accessed within a specified time period. This prevents the cache from growing indefinitely and ensures that templates are periodically refreshed.
*   **Thread Safety:**  The cache is protected by a `RwLock`, allowing concurrent read access while providing exclusive write access for template updates. This ensures thread safety and prevents data corruption.

## Implementation Details

*   **Cache Structure:** The template cache is implemented as a `HashMap<String, CachedTemplate>`, where:
    *   `String` is the file path of the MJML template.
    *   `CachedTemplate` is a struct containing:
        *   `content`: The template content (as a String).
        *   `last_accessed`:  An `Instant` value representing the last time the template was accessed.
*   **Cache Cleaning:** A background task runs periodically (every 10 minutes by default) and removes expired templates from the cache.
*   **Expiration Duration:** The expiration duration is configurable (defaulting to 1 hour). Templates that haven't been accessed within this duration are considered expired.
*   **Concurrency:** A mutex ensures that read and writes to the cache are thread safe.

## Configuration

The following parameters control the behavior of the template cache:

*   **Template Directory:** The directory where MJML templates are stored.
*   **Cache Cleaning Interval:** The frequency at which the background cache cleaning task runs (default: 10 minutes).
*   **Expiration Duration:** The duration after which a template is considered expired (default: 1 hour).

## Usage

The template cache is automatically managed by the server. There is no need to manually interact with the cache in most cases.

## Monitoring

The size of the template cache is logged to the console whenever a new template is added or when the cache is cleaned. Monitoring these logs can provide insights into the cache's behavior and help you optimize the cache settings.

## Benefits

*   **Reduced Latency:** Serving templates from memory significantly reduces the latency of MJML to HTML conversions.
*   **Improved Throughput:** By avoiding disk I/O and template recompilation, the server can handle more requests concurrently.

## Potential Improvements

*   **Faster Development Cycles:** Hot reloading allows developers to quickly iterate on templates without restarting the server.
*   **LRU (Least Recently Used) Eviction Policy:** Implement an LRU eviction policy to prioritize the caching of the most frequently used templates.
*   **Cache Size Limit:** Add a maximum size limit to the cache to prevent it from consuming too much memory.
*   **Metrics:** Expose metrics for cache hit rate and eviction counts to facilitate more detailed monitoring and optimization.



## Contributing

(Contribution guidelines would go here)
