# RAI Endpoint Simulator

Sebuah simulator endpoint OpenAI yang dibangun dengan Rust menggunakan framework Actix-web. Aplikasi ini mensimulasikan API chat completions OpenAI dengan dukungan streaming response dan dapat mengambil data dari file markdown atau database ClickHouse.

## 🚀 Fitur

- **Simulasi OpenAI Chat Completions API** - Endpoint `/v1/chat/completions` yang kompatibel
- **Streaming Response** - Dukungan Server-Sent Events (SSE) untuk streaming chunks
- **Dual Data Source** - Mendukung sumber data dari file markdown atau database ClickHouse
- **Redis Caching** - High-performance caching dengan Redis untuk response database dan file
- **Rate Limiting** - Menggunakan semaphore untuk mengontrol concurrent requests
- **Configurable Workers** - Jumlah worker threads dapat dikonfigurasi
- **Logging Konfigurabel** - Multiple level logging (trace, debug, info, warn, error)
- **Docker Support** - Dockerfile optimized dengan static linking
- **Test Endpoint** - Endpoint testing untuk verifikasi konektivitas

## 📋 Persyaratan

- Rust 1.70+ (latest stable)
- Redis 6+ (untuk caching)
- ClickHouse database (jika menggunakan database source)
- Docker (optional, untuk containerization)

## 🛠️ Instalasi

### Manual Setup

1. **Clone repository**
   ```bash
   git clone https://github.com/fullstack-aidev/rai-endpoint-simulator.git
   cd rai-endpoint-simulator
   ```

2. **Install dependencies**
   ```bash
   cargo build --release
   ```

3. **Start Redis**
   ```bash
   # Using Docker
   docker run -d --name redis -p 6379:6379 redis:7-alpine
   
   # Or install locally
   redis-server
   ```

4. **Setup konfigurasi**
   
   Copy `config.local.yml` ke `config.yml` untuk development lokal:
   ```bash
   cp config.local.yml config.yml
   ```

5. **Persiapkan response files** (jika menggunakan file source)
   
   Buat folder `zresponse` dan isi dengan file markdown (.md) yang berisi response content.

6. **Run**
   ```bash
   cargo run --release
   ```

### Docker Setup

1. **Run dengan Docker Compose** (recommended)
   ```bash
   docker-compose up -d
   ```
   
   Ini akan menjalankan:
   - Redis server
   - 3 replicas simulator
   - HAProxy load balancer

2. **Build image manual**
   ```bash
   docker build -t rai-endpoint-simulator .
   ```

3. **Run container manual**
   ```bash
   docker run -p 4545:4545 \
     -v $(pwd)/config.yml:/app/config.yml \
     -v $(pwd)/zresponse:/app/zresponse \
     rai-endpoint-simulator
   ```

## 🔧 Konfigurasi

### File config.yml

```yaml
binding:
  port: 4545
  host: 0.0.0.0
source: file # file or database
database:
  url: http://127.0.0.1:8123
  username: simulator_app
  password: "your_password"
redis:
  url: redis://127.0.0.1:6379
  prefix: rai_simulator
tracking:
  enabled: false
log_level: info
channel_capacity: 1000
semaphore_limit: 10000
workers: 8
cache_ttl: 60
```

### Parameter Konfigurasi

| Parameter | Deskripsi | Default |
|-----------|-----------|---------|
| `source` | Sumber data: "file" atau "database" | "file" |
| `log_level` | Level logging: trace/debug/info/warn/error | "info" |
| `channel_capacity` | Kapasitas channel untuk streaming | 1000 |
| `semaphore_limit` | Limit concurrent requests | 10000 |
| `workers` | Jumlah worker threads | 8 |
| `cache_ttl` | Cache TTL dalam detik | 60 |
| `binding.host` | Host binding server | "0.0.0.0" |
| `binding.port` | Port server | 4545 |
| `database.username` | Username ClickHouse | - |
| `database.password` | Password ClickHouse | - |
| `database.url` | URL ClickHouse | - |
| `redis.url` | URL Redis server | "redis://127.0.0.1:6379" |
| `redis.prefix` | Prefix untuk Redis keys | "rai_simulator" |
| `tracking.enabled` | Enable detailed logging | false |

### Redis Configuration

Aplikasi menggunakan Redis untuk caching dengan struktur key berikut:

| Key Pattern | Deskripsi | TTL |
|-------------|-----------|-----|
| `{prefix}:db_responses` | Cache responses dari database | `cache_ttl` |
| `{prefix}:file:{filename}` | Cache konten file markdown | `cache_ttl` |
| `{prefix}:file_list` | Cache daftar file markdown | 600s (10 menit) |

### Database Schema (ClickHouse)

Jika menggunakan database source, pastikan tabel `response_simulator` memiliki struktur:

```sql
CREATE TABLE response_simulator (
    qa_id UUID,
    pertanyaan String,
    jawaban String,
    referensi String
) ENGINE = MergeTree()
ORDER BY qa_id;
```

## 🚀 Penggunaan

### Menjalankan Server

```bash
# Development
cargo run

# Production
cargo run --release
```

Server akan berjalan di `http://localhost:4545`

### Endpoints

#### 1. Test Endpoint
```bash
POST /test_completion
```

Response untuk testing konektivitas:
```json
{
  "id": "chatcmpl-AjoahzpVUCsJmOQZRKZUze7qBjEjn",
  "object": "chat.completion",
  "created": 1735482595,
  "model": "gpt-4o-2024-08-06",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant", 
      "content": "============>>  Selamat! Aplikasi anda telah sukses terhubung ke OpenAI Simulator. <============="
    },
    "logprobs": null,
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 57,
    "completion_tokens": 92, 
    "total_tokens": 149
  }
}
```

#### 2. Chat Completions (Streaming)
```bash
POST /v1/chat/completions
Content-Type: application/json

{
  "model": "gpt-4o-2024-08-06",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "stream": true
}
```

Response streaming dalam format Server-Sent Events dengan chunks yang mensimulasikan response OpenAI.

### Contoh Penggunaan dengan cURL

```bash
# Test endpoint
curl -X POST http://localhost:4545/test_completion

# Chat completions
curl -X POST http://localhost:4545/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4o-2024-08-06", "messages": [{"role": "user", "content": "Hello!"}], "stream": true}'
```

## 📁 Struktur Project

```
rai-endpoint-simulator/
├── src/
│   ├── main.rs              # Entry point dan HTTP handlers
│   ├── stream.rs            # Streaming logic dan chunk generation
│   ├── response.rs          # File dan database response handling
│   └── config_loader.rs     # Configuration loading
├── zresponse/               # Markdown response files (jika source=file)
├── config.yml               # Konfigurasi aplikasi (Docker)
├── config.local.yml         # Konfigurasi aplikasi (Local development)
├── docker-compose.yaml      # Docker Compose configuration
├── haproxy.cfg              # HAProxy load balancer config
├── Cargo.toml               # Rust dependencies
├── Cargo.lock               # Locked dependencies
└── Dockerfile               # Container configuration
```

## ⚡ Performance Optimizations

### Redis Caching
- **Database responses** di-cache di Redis dengan TTL yang dapat dikonfigurasi
- **File content** di-cache untuk menghindari disk I/O berulang
- **File list** di-cache dengan TTL lebih lama (10 menit)
- Menggunakan `ConnectionManager` untuk connection pooling ke Redis

### Async I/O
- Menggunakan `tokio::task::spawn_blocking` untuk file I/O
- Non-blocking Redis operations dengan `redis::aio`
- Async database queries dengan ClickHouse async client

### Concurrency
- Configurable worker threads via `workers` config
- Semaphore-based rate limiting
- Lock-free caching dengan Redis sebagai distributed cache

### Memory Efficiency
- Streaming response tanpa buffering seluruh content
- Efficient chunk generation dengan configurable chunk size

## 🔍 Monitoring dan Debugging

### Logging

Aplikasi menggunakan `env_logger` dengan level yang dapat dikonfigurasi:

```yaml
# Set log level via config.yml
log_level: "debug"
```

Log output contoh:
```
[INFO] Starting server at http://0.0.0.0:4545
[INFO] Configuration: workers=8, semaphore_limit=10000, cache_ttl=60s
[INFO] Connecting to Redis at redis://127.0.0.1:6379
[INFO] Successfully connected to Redis
[DEBUG] Cache hit: returning 150 cached responses from Redis
```

### Health Check

Gunakan test endpoint untuk health checking:

```bash
curl -f http://localhost:4545/test_completion || exit 1
```

### Redis Monitoring

```bash
# Connect ke Redis CLI
redis-cli

# Lihat semua keys
KEYS rai_simulator:*

# Lihat TTL
TTL rai_simulator:db_responses

# Monitor cache activity
MONITOR
```

## 🐳 Docker Compose Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      HAProxy                             │
│                    (Port 4545)                           │
└─────────────────────┬───────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│ Replica 1 │ │ Replica 2 │ │ Replica 3 │
└─────┬─────┘ └─────┬─────┘ └─────┬─────┘
      │             │             │
      └─────────────┼─────────────┘
                    │
                    ▼
            ┌───────────────┐
            │     Redis     │
            │  (Shared Cache)│
            └───────────────┘
```

## 🤝 Contributing

1. Fork repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

## 📝 License

Distributed under the MIT License. See `LICENSE` file for more information.

## 🆘 Support

Jika Anda mengalami masalah atau memiliki pertanyaan:

1. Check existing [Issues](https://github.com/fullstack-aidev/rai-endpoint-simulator/issues)
2. Create new issue dengan detail lengkap
3. Sertakan log output dan konfigurasi (redact sensitive information)

## 🚀 Deployment

### Production Considerations

1. **Security**: Pastikan Redis dan database credentials aman
2. **Redis**: Gunakan Redis dengan persistence (AOF/RDB) untuk production
3. **Monitoring**: Setup logging, metrics, dan alerting
4. **Scaling**: Adjust `workers` dan `semaphore_limit` sesuai kapasitas server
5. **Backup**: Backup database dan response files secara berkala

### Recommended Production Config

```yaml
binding:
  port: 4545
  host: 0.0.0.0
source: file
redis:
  url: redis://your-redis-cluster:6379
  prefix: rai_prod
log_level: warn
channel_capacity: 1000
semaphore_limit: 5000
workers: 8
cache_ttl: 300
tracking:
  enabled: false
```

---

**Built with ❤️ using Rust and Actix-web**