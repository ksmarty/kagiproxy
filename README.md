# KagiProxy

OpenAI-compatible REST API proxy for Kagi Assistant.

## Build Instructions

```bash
cargo build --release
```

## Docker Instructions

```bash
docker compose up --build -d
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `KAGI_SESSION_TOKEN` | Kagi session token for authentication | - |
| `KAGI_AUTH_HEADER` | Custom header name for auth (e.g., "Authorization") | "Authorization" |
| `KAGI_BASE_URL` | Kagi API base URL | "https://kagi.com" |
| `PORT` | Server port | "3000" |
| `RUST_LOG` | Logging level | "info" |

## Example curl Commands

### Non-streaming chat completion:
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'
```

### Streaming chat completion:
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": true
  }'
```

### List models:
```bash
curl http://localhost:3000/v1/models
```

### Health check:
```bash
curl http://localhost:3000/health
```

## Endpoints

- `GET /health` - Health check
- `GET /v1/models` - List available models
- `POST /v1/chat/completions` - Chat completions (streaming and non-streaming)