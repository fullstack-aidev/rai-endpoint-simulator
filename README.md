# RAI Endpoint Simulator

Sebuah simulator endpoint OpenAI yang dibangun dengan Rust menggunakan framework Actix-web. Aplikasi ini mensimulasikan API chat completions OpenAI dengan dukungan streaming response dan dapat mengambil data dari file markdown atau database ClickHouse.

## 🚀 Fitur

- **Simulasi OpenAI Chat Completions API** - Endpoint `/v1/chat/completions` yang kompatibel
- **Streaming Response** - Dukungan Server-Sent Events (SSE) untuk streaming chunks
- **Dual Data Source** - Mendukung sumber data dari file markdown atau database ClickHouse
- **Rate Limiting** - Menggunakan semaphore untuk mengontrol concurrent requests
- **Logging Konfigurabel** - Multiple level logging (trace, debug, info, warn, error)
- **Docker Support** - Dockerfile optimized dengan static linking
- **Test Endpoint** - Endpoint testing untuk verifikasi konektivitas

## 📋 Persyaratan

- Rust 1.70+ (latest stable)
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

3. **Setup konfigurasi**
   
   Buat file `config.yml` berdasarkan template berikut:
   ```yaml
   source: "file"  # atau "database"
   log_level: "info"
   channel_capacity: 1000
   semaphore_limit: 100
   
   binding:
     host: "0.0.0.0"
     port: 8080
   
   database:
     username: "your_username"
     password: "your_password" 
     url: "http://localhost:8123"
   
   tracking:
     enabled: true
   ```

4. **Persiapkan response files** (jika menggunakan file source)
   
   Buat folder `zresponse` dan isi dengan file markdown (.md) yang berisi response content.

### Docker Setup

1. **Build image**
   ```bash
   docker build -t rai-endpoint-simulator .
   ```

2. **Run container**
   ```bash
   docker run -p 8080:8080 -v $(pwd)/config.yml:/app/config.yml -v $(pwd)/zresponse:/app/zresponse rai-endpoint-simulator
   ```

## 🔧 Konfigurasi

### File config.yml

| Parameter | Deskripsi | Default |
|-----------|-----------|---------|
| `source` | Sumber data: "file" atau "database" | "file" |
| `log_level` | Level logging: trace/debug/info/warn/error | "info" |
| `channel_capacity` | Kapasitas channel untuk streaming | 1000 |
| `semaphore_limit` | Limit concurrent requests | 100 |
| `binding.host` | Host binding server | "0.0.0.0" |
| `binding.port` | Port server | 8080 |
| `database.username` | Username ClickHouse | - |
| `database.password` | Password ClickHouse | - |
| `database.url` | URL ClickHouse | - |
| `tracking.enabled` | Enable detailed logging | true |

### Database Schema (ClickHouse)

Jika menggunakan database source, pastikan tabel `response_simulator` memiliki struktur:

```sql
CREATE TABLE response_simulator (
    qa_id String,
    pertanyaan String,
    jawaban String,
    referensi String
) ENGINE = MergeTree()
ORDER BY qa_id;
```

## 🚀 Penggunaan

### Menjalankan Server

```bash
cargo run
```

Server akan berjalan di `http://localhost:8080`

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
curl -X POST http://localhost:8080/test_completion

# Chat completions
curl -X POST http://localhost:8080/v1/chat/completions \
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
├── config.yml               # Konfigurasi aplikasi
├── Cargo.toml              # Rust dependencies
├── Cargo.lock              # Locked dependencies
└── Dockerfile              # Container configuration
```

## 🔍 Monitoring dan Debugging

### Logging

Aplikasi menggunakan `env_logger` dengan level yang dapat dikonfigurasi:

```bash
# Set log level via config.yml
log_level: "debug"
```

### Health Check

Gunakan test endpoint untuk health checking:

```bash
curl -f http://localhost:8080/test_completion || exit 1
```

## ⚡ Performance

- **Concurrent Requests**: Dikontrol oleh `semaphore_limit` dalam config
- **Memory Usage**: Optimized dengan static linking dan musl target
- **Response Time**: Chunk streaming dengan delay konfigurabel
- **Database Pooling**: ClickHouse client dengan connection management

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

1. **Security**: Pastikan database credentials aman
2. **Monitoring**: Setup logging dan monitoring
3. **Scaling**: Adjust `semaphore_limit` sesuai kapasitas server
4. **Backup**: Backup database dan response files secara berkala

### Environment Variables

Aplikasi dapat dikonfigurasi via environment variables untuk deployment:

```bash
export RAI_LOG_LEVEL=info
export RAI_PORT=8080
export RAI_DB_URL=http://your-clickhouse:8123
```

---

**Built with ❤️ using Rust and Actix-web**
