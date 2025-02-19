# MUSL MRML Server 

This project renders MJML templates using Handlebars.

## Dependencies

*   `axum`: Web framework
*   `tokio`: Asynchronous runtime
*   `warp`: Another web framework (possibly used for specific features)
*   `handlebars`: Templating engine
*   `mrml`: MJML library
*   `select`: HTML parser
*   `serde` and `serde_json`: JSON serialization/deserialization

## Build And Run

```bash
docker-compose up -d
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


## Contributing

(Contribution guidelines would go here)
