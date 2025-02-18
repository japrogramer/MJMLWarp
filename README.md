# mrml_template_renderer

This project renders MJML templates using Handlebars.

## Dependencies

*   `axum`: Web framework
*   `tokio`: Asynchronous runtime
*   `warp`: Another web framework (possibly used for specific features)
*   `handlebars`: Templating engine
*   `mrml`: MJML library
*   `select`: HTML parser
*   `serde` and `serde_json`: JSON serialization/deserialization

## Usage
(Instructions on how to run the application would go here)

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


### List Templates

```bash
curl http://localhost:3030/templates
```

### Upload Template

```bash
curl -X POST \
  -F "file=@./my-template.mjml" \
  http://localhost:3030/templates
```

### List Templates

```bash
curl http://localhost:3030/templates
```

### Upload Template

```bash
curl -X POST \
  -F "file=@./my-template.mjml" \
  http://localhost:3030/templates
```


## Contributing

(Contribution guidelines would go here)