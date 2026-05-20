# KagiProxy

OpenAI-compatible REST API proxy for Kagi Assistant.

## Quick Start (Docker)

### Pull from GHCR (recommended)

```bash
docker pull ghcr.io/ksmarty/kagiproxy:latest
docker run -d -p 3000:3000 \
  -e KAGI_SESSION_TOKEN=your_token \
  ghcr.io/ksmarty/kagiproxy:latest
```

Or with docker-compose:

```yaml
services:
  kagi-proxy:
    image: ghcr.io/ksmarty/kagiproxy:latest
    ports:
      - "3000:3000"
    environment:
      - KAGI_SESSION_TOKEN=${KAGI_SESSION_TOKEN}
```

### Build locally

```bash
docker compose up --build -d
```

## Build from Source

```bash
cargo build --release
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