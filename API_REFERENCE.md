# RAI Endpoint Simulator - API Reference

Dokumentasi lengkap API endpoints untuk RAI Endpoint Simulator dengan contoh implementasi client menggunakan Go.

## 📋 Base Information

- **Base URL**: `http://localhost:8080` (default)
- **Content-Type**: `application/json`
- **Response Format**: JSON atau Server-Sent Events (SSE)

## 🔗 Endpoints

### 1. Test Connection Endpoint

Endpoint untuk memverifikasi konektivitas dan status server.

#### Request

```http
POST /test_completion
```

#### Go Client Example

```go
package main

import (
    "encoding/json"
    "fmt"
    "io/ioutil"
    "net/http"
    "time"
)

type TestResponse struct {
    ID      string `json:"id"`
    Object  string `json:"object"`
    Created int64  `json:"created"`
    Model   string `json:"model"`
    Choices []struct {
        Index   int `json:"index"`
        Message struct {
            Role    string `json:"role"`
            Content string `json:"content"`
        } `json:"message"`
        LogProbs     interface{} `json:"logprobs"`
        FinishReason string      `json:"finish_reason"`
    } `json:"choices"`
    Usage struct {
        PromptTokens     int `json:"prompt_tokens"`
        CompletionTokens int `json:"completion_tokens"`
        TotalTokens      int `json:"total_tokens"`
    } `json:"usage"`
}

func TestConnection(baseURL string) (*TestResponse, error) {
    client := &http.Client{
        Timeout: 30 * time.Second,
    }

    resp, err := client.Post(baseURL+"/test_completion", "application/json", nil)
    if err != nil {
        return nil, fmt.Errorf("failed to make request: %w", err)
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
    }

    body, err := ioutil.ReadAll(resp.Body)
    if err != nil {
        return nil, fmt.Errorf("failed to read response: %w", err)
    }

    var testResp TestResponse
    if err := json.Unmarshal(body, &testResp); err != nil {
        return nil, fmt.Errorf("failed to unmarshal response: %w", err)
    }

    return &testResp, nil
}

func main() {
    baseURL := "http://localhost:8080"
    
    resp, err := TestConnection(baseURL)
    if err != nil {
        fmt.Printf("Connection test failed: %v\n", err)
        return
    }
    
    fmt.Printf("Connection successful!\n")
    fmt.Printf("Response: %s\n", resp.Choices[0].Message.Content)
}
```

#### Response Format

```json
{
  "id": "chatcmpl-AjoahzpVUCsJmOQZRKZUze7qBjEjn",
  "object": "chat.completion",
  "created": 1735482595,
  "model": "gpt-4o-2024-08-06",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "============>>  Selamat! Aplikasi anda telah sukses terhubung ke OpenAI Simulator. <============="
      },
      "logprobs": null,
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 57,
    "completion_tokens": 92,
    "total_tokens": 149
  }
}
```

### 2. Chat Completions Endpoint

Endpoint utama untuk simulasi chat completions dengan streaming response.

#### Request

```http
POST /v1/chat/completions
Content-Type: application/json
```

#### Request Body Schema

```go
type ChatCompletionRequest struct {
    Model       string    `json:"model"`
    Messages    []Message `json:"messages"`
    Stream      bool      `json:"stream,omitempty"`
    MaxTokens   int       `json:"max_tokens,omitempty"`
    Temperature float64   `json:"temperature,omitempty"`
    TopP        float64   `json:"top_p,omitempty"`
    N           int       `json:"n,omitempty"`
    Stop        []string  `json:"stop,omitempty"`
    User        string    `json:"user,omitempty"`
}

type Message struct {
    Role    string `json:"role"`    // "system", "user", "assistant"
    Content string `json:"content"`
}
```

#### Go Streaming Client Example

```go
package main

import (
    "bufio"
    "bytes"
    "encoding/json"
    "fmt"
    "io"
    "net/http"
    "strings"
    "time"
)

type StreamChunk struct {
    ID                string `json:"id"`
    Object            string `json:"object"`
    Created           int64  `json:"created"`
    Model             string `json:"model"`
    SystemFingerprint string `json:"system_fingerprint"`
    Choices           []struct {
        Index  int `json:"index"`
        Delta  struct {
            Content string `json:"content"`
        } `json:"delta"`
        LogProbs     interface{} `json:"logprobs"`
        FinishReason *string     `json:"finish_reason"`
    } `json:"choices"`
    Usage *struct {
        PromptTokens     int `json:"prompt_tokens"`
        CompletionTokens int `json:"completion_tokens"`
        TotalTokens      int `json:"total_tokens"`
        PromptTokensDetails struct {
            CachedTokens int `json:"cached_tokens"`
            AudioTokens  int `json:"audio_tokens"`
        } `json:"prompt_tokens_details"`
        CompletionTokensDetails struct {
            ReasoningTokens            int `json:"reasoning_tokens"`
            AudioTokens                int `json:"audio_tokens"`
            AcceptedPredictionTokens   int `json:"accepted_prediction_tokens"`
            RejectedPredictionTokens   int `json:"rejected_prediction_tokens"`
        } `json:"completion_tokens_details"`
    } `json:"usage,omitempty"`
}

type RAIClient struct {
    BaseURL    string
    HTTPClient *http.Client
}

func NewRAIClient(baseURL string) *RAIClient {
    return &RAIClient{
        BaseURL: baseURL,
        HTTPClient: &http.Client{
            Timeout: 300 * time.Second, // Extended timeout for streaming
        },
    }
}

func (c *RAIClient) ChatCompletions(req ChatCompletionRequest) (<-chan StreamChunk, <-chan error) {
    chunkChan := make(chan StreamChunk, 100)
    errorChan := make(chan error, 1)

    go func() {
        defer close(chunkChan)
        defer close(errorChan)

        reqBody, err := json.Marshal(req)
        if err != nil {
            errorChan <- fmt.Errorf("failed to marshal request: %w", err)
            return
        }

        httpReq, err := http.NewRequest("POST", c.BaseURL+"/v1/chat/completions", bytes.NewReader(reqBody))
        if err != nil {
            errorChan <- fmt.Errorf("failed to create request: %w", err)
            return
        }

        httpReq.Header.Set("Content-Type", "application/json")
        httpReq.Header.Set("Accept", "text/event-stream")

        resp, err := c.HTTPClient.Do(httpReq)
        if err != nil {
            errorChan <- fmt.Errorf("failed to make request: %w", err)
            return
        }
        defer resp.Body.Close()

        if resp.StatusCode != http.StatusOK {
            errorChan <- fmt.Errorf("unexpected status code: %d", resp.StatusCode)
            return
        }

        scanner := bufio.NewScanner(resp.Body)
        for scanner.Scan() {
            line := scanner.Text()
            
            // Skip empty lines and non-data lines
            if line == "" || !strings.HasPrefix(line, "data: ") {
                continue
            }

            // Extract JSON data
            data := strings.TrimPrefix(line, "data: ")
            if data == "[DONE]" {
                break
            }

            var chunk StreamChunk
            if err := json.Unmarshal([]byte(data), &chunk); err != nil {
                // Skip malformed chunks, don't break the stream
                continue
            }

            chunkChan <- chunk
        }

        if err := scanner.Err(); err != nil {
            errorChan <- fmt.Errorf("error reading stream: %w", err)
        }
    }()

    return chunkChan, errorChan
}

// Non-streaming version for simpler use cases
func (c *RAIClient) ChatCompletionsSync(req ChatCompletionRequest) (string, error) {
    req.Stream = true // Force streaming for this simulator
    
    chunkChan, errorChan := c.ChatCompletions(req)
    var fullResponse strings.Builder

    for {
        select {
        case chunk, ok := <-chunkChan:
            if !ok {
                return fullResponse.String(), nil
            }
            
            if len(chunk.Choices) > 0 {
                fullResponse.WriteString(chunk.Choices[0].Delta.Content)
            }
            
        case err := <-errorChan:
            if err != nil {
                return "", err
            }
        }
    }
}

func main() {
    client := NewRAIClient("http://localhost:8080")

    // Example request
    req := ChatCompletionRequest{
        Model: "gpt-4o-2024-08-06",
        Messages: []Message{
            {
                Role:    "system",
                Content: "You are a helpful assistant.",
            },
            {
                Role:    "user", 
                Content: "Explain quantum computing in simple terms.",
            },
        },
        Stream:      true,
        MaxTokens:   500,
        Temperature: 0.7,
    }

    fmt.Println("Starting streaming chat completion...")
    
    chunkChan, errorChan := client.ChatCompletions(req)
    var fullResponse strings.Builder

    for {
        select {
        case chunk, ok := <-chunkChan:
            if !ok {
                fmt.Printf("\n\nFull response:\n%s\n", fullResponse.String())
                return
            }
            
            if len(chunk.Choices) > 0 {
                content := chunk.Choices[0].Delta.Content
                fmt.Print(content)
                fullResponse.WriteString(content)
            }
            
            // Check for completion
            if chunk.Usage != nil {
                fmt.Printf("\n\nUsage: %+v\n", chunk.Usage)
            }
            
        case err := <-errorChan:
            if err != nil {
                fmt.Printf("Error: %v\n", err)
                return
            }
        }
    }
}
```

#### Response Format (Streaming)

Setiap chunk dalam format:

```
data: {"id":"chatcmpl-Ai...","object":"chat.completion.chunk","created":1735278816,"model":"gpt-4o-2024-08-06","system_fingerprint":"fp_d28bcae782","choices":[{"index":0,"delta":{"content":"Hello"},"logprobs":null,"finish_reason":null}],"usage":null}

```

Final chunk dengan usage:

```
data: {"id":"chatcmpl-Ai...","object":"chat.completion.chunk","created":1735278816,"model":"gpt-4o-2024-08-06","system_fingerprint":"fp_d28bcae782","choices":[],"usage":{"prompt_tokens":182,"completion_tokens":520,"total_tokens":702,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0,"audio_tokens":0,"accepted_prediction_tokens":0,"rejected_prediction_tokens":0}}}

```

## 🛠️ Advanced Go Client with Features

```go
package raiclient

import (
    "bufio"
    "bytes"
    "context"
    "encoding/json"
    "fmt"
    "net/http"
    "strings"
    "time"
)

type Config struct {
    BaseURL    string
    Timeout    time.Duration
    MaxRetries int
    Debug      bool
}

type Client struct {
    config     Config
    httpClient *http.Client
}

func NewClient(config Config) *Client {
    if config.Timeout == 0 {
        config.Timeout = 300 * time.Second
    }
    if config.MaxRetries == 0 {
        config.MaxRetries = 3
    }

    return &Client{
        config: config,
        httpClient: &http.Client{
            Timeout: config.Timeout,
        },
    }
}

// Health check
func (c *Client) HealthCheck(ctx context.Context) error {
    req, err := http.NewRequestWithContext(ctx, "POST", c.config.BaseURL+"/test_completion", nil)
    if err != nil {
        return err
    }

    resp, err := c.httpClient.Do(req)
    if err != nil {
        return err
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return fmt.Errorf("health check failed with status: %d", resp.StatusCode)
    }

    return nil
}

// Chat completion with context and retry logic
func (c *Client) ChatCompletionWithContext(ctx context.Context, req ChatCompletionRequest) (<-chan StreamChunk, <-chan error) {
    chunkChan := make(chan StreamChunk, 100)
    errorChan := make(chan error, 1)

    go func() {
        defer close(chunkChan)
        defer close(errorChan)

        var lastErr error
        for attempt := 0; attempt < c.config.MaxRetries; attempt++ {
            if attempt > 0 {
                if c.config.Debug {
                    fmt.Printf("Retrying request (attempt %d/%d)\n", attempt+1, c.config.MaxRetries)
                }
                
                select {
                case <-ctx.Done():
                    errorChan <- ctx.Err()
                    return
                case <-time.After(time.Duration(attempt) * time.Second):
                }
            }

            err := c.performRequest(ctx, req, chunkChan)
            if err == nil {
                return // Success
            }
            
            lastErr = err
            if c.config.Debug {
                fmt.Printf("Request failed (attempt %d): %v\n", attempt+1, err)
            }
        }

        errorChan <- fmt.Errorf("max retries exceeded, last error: %w", lastErr)
    }()

    return chunkChan, errorChan
}

func (c *Client) performRequest(ctx context.Context, req ChatCompletionRequest, chunkChan chan<- StreamChunk) error {
    reqBody, err := json.Marshal(req)
    if err != nil {
        return fmt.Errorf("failed to marshal request: %w", err)
    }

    httpReq, err := http.NewRequestWithContext(ctx, "POST", c.config.BaseURL+"/v1/chat/completions", bytes.NewReader(reqBody))
    if err != nil {
        return fmt.Errorf("failed to create request: %w", err)
    }

    httpReq.Header.Set("Content-Type", "application/json")
    httpReq.Header.Set("Accept", "text/event-stream")

    resp, err := c.httpClient.Do(httpReq)
    if err != nil {
        return fmt.Errorf("failed to make request: %w", err)
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return fmt.Errorf("unexpected status code: %d", resp.StatusCode)
    }

    scanner := bufio.NewScanner(resp.Body)
    for scanner.Scan() {
        select {
        case <-ctx.Done():
            return ctx.Err()
        default:
        }

        line := scanner.Text()
        if line == "" || !strings.HasPrefix(line, "data: ") {
            continue
        }

        data := strings.TrimPrefix(line, "data: ")
        if data == "[DONE]" {
            break
        }

        var chunk StreamChunk
        if err := json.Unmarshal([]byte(data), &chunk); err != nil {
            if c.config.Debug {
                fmt.Printf("Failed to unmarshal chunk: %v\n", err)
            }
            continue
        }

        select {
        case chunkChan <- chunk:
        case <-ctx.Done():
            return ctx.Err()
        }
    }

    return scanner.Err()
}
```

## 📝 Example Usage Scenarios

### Simple Chat Bot

```go
func main() {
    client := NewRAIClient("http://localhost:8080")
    
    for {
        fmt.Print("You: ")
        var input string
        fmt.Scanln(&input)
        
        if input == "quit" {
            break
        }
        
        req := ChatCompletionRequest{
            Model: "gpt-4o-2024-08-06",
            Messages: []Message{
                {Role: "user", Content: input},
            },
            Stream: true,
        }
        
        fmt.Print("AI: ")
        response, err := client.ChatCompletionsSync(req)
        if err != nil {
            fmt.Printf("Error: %v\n", err)
            continue
        }
        
        fmt.Printf("%s\n\n", response)
    }
}
```

### Batch Processing

```go
func processBatch(client *RAIClient, questions []string) {
    for i, question := range questions {
        fmt.Printf("Processing question %d/%d: %s\n", i+1, len(questions), question)
        
        req := ChatCompletionRequest{
            Model: "gpt-4o-2024-08-06",
            Messages: []Message{
                {Role: "user", Content: question},
            },
            Stream: true,
        }
        
        response, err := client.ChatCompletionsSync(req)
        if err != nil {
            fmt.Printf("Error processing question %d: %v\n", i+1, err)
            continue
        }
        
        fmt.Printf("Response %d: %s\n\n", i+1, response)
    }
}
```

## 🚨 Error Handling

### Common Error Codes

| Status Code | Description | Go Error Handling |
|-------------|-------------|-------------------|
| 200 | Success | Normal processing |
| 400 | Bad Request | Check request format |
| 429 | Rate Limited | Implement backoff |
| 500 | Server Error | Retry with exponential backoff |

### Error Handling Example

```go
func handleStreamingWithRetry(client *RAIClient, req ChatCompletionRequest, maxRetries int) (string, error) {
    var lastErr error
    
    for attempt := 0; attempt < maxRetries; attempt++ {
        if attempt > 0 {
            time.Sleep(time.Duration(attempt) * time.Second)
        }
        
        response, err := client.ChatCompletionsSync(req)
        if err == nil {
            return response, nil
        }
        
        lastErr = err
        fmt.Printf("Attempt %d failed: %v\n", attempt+1, err)
    }
    
    return "", fmt.Errorf("failed after %d attempts: %w", maxRetries, lastErr)
}
```

## 🔧 Configuration Examples

### Production Configuration

```go
config := Config{
    BaseURL:    "https://your-rai-simulator.com",
    Timeout:    60 * time.Second,
    MaxRetries: 5,
    Debug:      false,
}

client := NewClient(config)
```

### Development Configuration

```go
config := Config{
    BaseURL:    "http://localhost:8080",
    Timeout:    300 * time.Second,
    MaxRetries: 3,
    Debug:      true,
}

client := NewClient(config)
```

---

**📚 Dokumentasi ini memberikan semua yang dibutuhkan untuk mengintegrasikan aplikasi Go dengan RAI Endpoint Simulator.**
